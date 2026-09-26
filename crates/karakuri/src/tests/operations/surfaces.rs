use super::*;

/// Verifies command-line argument parsing requires either zero or two paths, rejecting single-path inputs.
#[test]
fn a_set_is_two_paths_or_none_and_anything_else_is_refused() {
    let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));

    let bare = of(&[]).expect("no arguments is the pair the preset library ships");
    assert_eq!(bare.sources.l1, shipped().l1);
    assert_eq!(bare.sources.l4, shipped().l4);
    assert!(
        bare.sources.l1.is_file() && bare.sources.l4.is_file(),
        "the default pair is not on the disk at {} and {}, so a bare run cannot draw",
        bare.sources.l1.display(),
        bare.sources.l4.display()
    );

    let named = of(&["a/geo.kir", "b/ren.kir"]).expect("two paths are a Set");
    assert_eq!(named.sources.l1, std::path::PathBuf::from("a/geo.kir"));
    assert_eq!(named.sources.l4, std::path::PathBuf::from("b/ren.kir"));
    assert_eq!(
        named.sources.material(),
        "geo + ren",
        "the strip is not named after what was actually loaded"
    );

    let one = of(&["a/geo.kir"]).expect_err(
        "one path was read as a Set, so this program would have invented the other half",
    );
    assert!(
        one.contains("a/geo.kir"),
        "the refusal `{one}` does not name the path it refused"
    );
    assert!(
        of(&["a.kir", "b.kir", "c.kir"]).is_err(),
        "three paths were read as a Set"
    );

    // Help flags return an empty error string so main can print usage and exit 0.
    assert_eq!(of(&["--help"]).err(), Some(String::new()));
    assert_eq!(of(&["-h"]).err(), Some(String::new()));
    assert!(
        !one.is_empty(),
        "a refusal came back with no sentence in it, which `main` reads as `--help`"
    );
}

/// Verifies `--store` and `--presets` flags can appear in any argument order, with explicit pairs taking precedence.
#[test]
fn the_two_flags_say_where_the_data_is_and_may_sit_on_either_side_of_the_pair() {
    let of = |args: &[&str]| sources_from(args.iter().map(|a| (*a).to_string()));
    let library = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let library = library
        .to_str()
        .expect("this workspace's path is not utf-8");

    // Verify default store path matches the shared constant.
    assert_eq!(
        of(&[]).expect("a bare run").store,
        std::path::PathBuf::from(karakuri_environment::places::STORE),
        "a run that said nothing about a store did not get the shared default"
    );

    for spelling in [
        vec!["--store", "/tmp/library", "a/geo.kir", "b/ren.kir"],
        vec!["a/geo.kir", "b/ren.kir", "--store", "/tmp/library"],
        vec!["a/geo.kir", "--store", "/tmp/library", "b/ren.kir"],
    ] {
        let launch = of(&spelling).unwrap_or_else(|why| panic!("{spelling:?}: {why}"));
        assert_eq!(
            launch.store,
            std::path::PathBuf::from("/tmp/library"),
            "{spelling:?} read a store nobody asked for"
        );
        assert_eq!(
            launch.sources.l1,
            std::path::PathBuf::from("a/geo.kir"),
            "{spelling:?} lost the pair to the flag"
        );
        assert_eq!(launch.sources.l4, std::path::PathBuf::from("b/ren.kir"));
    }

    // `--presets` with no pair: it is what the pair defaults to, and the
    // resolution reports it as typed rather than as something found.
    let told = of(&["--presets", library]).expect("a library that is there");
    assert_eq!(
        told.sources.l1,
        std::path::Path::new(library).join("coil_vortex.kir")
    );
    assert_eq!(
        told.sources.l4,
        std::path::Path::new(library).join("star_flares.kir")
    );
    assert_eq!(
        told.presets.as_ref().map(|presets| presets.found),
        Some(karakuri_environment::places::Found::Given),
        "a `--presets` an operator typed was reported as a place this program went \
         looking in"
    );

    // And with a pair, on either side: the pair wins and the library is
    // still the one that was named.
    for spelling in [
        vec!["--presets", library, "a/geo.kir", "b/ren.kir"],
        vec!["a/geo.kir", "b/ren.kir", "--presets", library],
    ] {
        let launch = of(&spelling).unwrap_or_else(|why| panic!("{spelling:?}: {why}"));
        assert_eq!(
            launch.sources.l1,
            std::path::PathBuf::from("a/geo.kir"),
            "{spelling:?}: `--presets` overrode the paths the operator named, which \
             would make it a second way of saying what plays"
        );
        assert_eq!(launch.sources.l4, std::path::PathBuf::from("b/ren.kir"));
        assert_eq!(
            launch.presets.map(|presets| presets.dir),
            Some(std::path::PathBuf::from(library)),
            "{spelling:?} lost the library it was given"
        );
    }

    // A `--presets` that is not there is refused rather than searched
    // past, and the sentence is `places`' own — one refusal, whichever
    // program the operator reached it from.
    let missing = std::path::Path::new(library).join("no-such-library");
    let why = of(&["--presets", missing.to_str().expect("utf-8")])
        .expect_err("a `--presets` that is not there was accepted");
    assert_eq!(why, karakuri_environment::places::no_presets_at(&missing));

    // A flag with nothing after it, and a flag whose value is the next
    // flag. Neither falls back and neither swallows.
    for (spelling, wanted) in [
        (vec!["--presets"], "`--presets` needs a value"),
        (vec!["--store"], "`--store` needs a value"),
        (
            vec!["--presets", "--store", "/tmp/library"],
            "`--presets` was given no value — `--store` is an option, not one",
        ),
    ] {
        assert_eq!(
            of(&spelling).as_ref().err().map(String::as_str),
            Some(wanted),
            "{spelling:?}"
        );
    }

    // Unknown options must be rejected rather than parsed as file arguments.
    let typo = of(&["--prests", library]).expect_err("an unknown option was read as half of a Set");
    assert_eq!(typo, "unknown option `--prests`");
    assert!(
        !typo.is_empty(),
        "a refusal came back with no sentence in it, which `main` reads as `--help`"
    );
}

/// Verifies slot capacity is derived directly from procedure header declarations rather than global defaults (ADR-0270).
#[test]
fn the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none() {
    let sources = shipped();

    // Pins reference workload L1 procedure name (`examples/drift_cloud.kset`).
    let reference = checked(&sources.l1.with_file_name("drift_shell.kir"));
    let pinned = reference
        .capacity
        .expect("an L1 that passed contract checking always carries a capacity");
    assert_eq!(
        pinned.default, 262_144,
        "`examples/drift_cloud.kset`'s L1 no longer declares the capacity every \
         host-clock figure in this repository was taken at, and \
         `docs/contributing.md` §1 names it as the one reference workload \
         (ADR-0270)"
    );

    // Verify capacity matches the file's own header declaration (ADR-0270).
    let l1 = checked(&sources.l1);
    let declared = l1
        .capacity
        .expect("an L1 that passed contract checking always carries a capacity");
    assert!(
        declared.contains(declared.default),
        "the file's own default is outside the range the same file declares"
    );

    assert_eq!(capacity_of(&l1), declared.default);

    assert!(
        checked(&sources.l4).capacity.is_none(),
        "the renderer declares a capacity — `Set::build` is handed the L1's, and \
         two declarations would be two answers to how many elements there are"
    );

    // Validates against procedures with non-default capacity declarations (`strand_shell.kir` at 131072) (ADR-0271).
    let other = checked(&sources.l1.with_file_name("strand_shell.kir"));
    assert_eq!(
        capacity_of(&other),
        131_072,
        "a second procedure did not run at what it declares — the capacity is being \
         read from somewhere other than the file"
    );
    assert_ne!(
        capacity_of(&other),
        karakuri_ir::DEFAULT_CAPACITY,
        "the second procedure declares the language default, so this test can no \
         longer tell a per-file read from a constant — pick another `.kir`"
    );
}

/// Verifies projector surface creation reuses the single `Gpu::instance()` initialized in `resumed` (ADR-0324).
#[test]
fn every_surface_is_made_by_the_instance_the_adapter_came_from() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut fresh = Vec::new();
    let mut surfaces = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "tests") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            let rel = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            for (i, line) in src.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                if code.contains("Gpu::instance()") {
                    fresh.push(format!("{rel}:{}", i + 1));
                }
                if code.contains("create_surface(") {
                    surfaces.push((format!("{rel}:{}", i + 1), code.trim().to_string()));
                }
            }
        }
    }
    assert_eq!(
        fresh,
        vec!["app/handler/mod.rs:49".to_string()],
        "a second wgpu::Instance would hold none of the first one's adapters: {fresh:?}"
    );
    assert!(!surfaces.is_empty(), "no surface is made anywhere");
    for (at, code) in &surfaces {
        assert!(
            code.contains("instance.create_surface(")
                || code.contains("gfx.gpu.instance.create_surface("),
            "{at}: a surface made off something other than the adapter's own instance: `{code}`"
        );
        assert!(
            !code.contains("Gpu::instance().create_surface("),
            "{at}: a surface on a fresh instance, whose adapter is another instance's: `{code}`"
        );
    }
}

/// Verifies presentation picture formats are queried dynamically from the console surface rather than hardcoding constants (ADR-0324, ADR-0361).
#[test]
fn a_picture_format_is_a_value_read_off_a_surface() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut named = Vec::new();
    let mut compared = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "tests") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            let rel = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            for (i, line) in src.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                for spelling in ["Rgba8UnormSrgb", "Bgra8UnormSrgb"] {
                    if code.contains(spelling) {
                        named.push(format!("{rel}:{}: `{}`", i + 1, code.trim()));
                    }
                }
                if code.contains("caps.formats.contains(") {
                    compared.push((format!("{rel}:{}", i + 1), code.trim().to_string()));
                }
            }
        }
    }
    assert!(
        named.is_empty(),
        "an 8-bit sRGB format named in the source is a guess about a display that Metal \
         already falsifies — read it off the surface instead: {named:?}"
    );
    assert_eq!(
        compared.len(),
        1,
        "the projector's format check is the one place a surface's formats are asked for a \
         member, and it has moved or multiplied: {compared:?}"
    );
    let (at, code) = &compared[0];
    assert!(
        at.starts_with("app/operations.rs"),
        "{at}: the projector's format check has moved out of `routed`: `{code}`"
    );
    assert!(
        code.contains("gfx.picture_format"),
        "{at}: the projector is checked against something other than the format the console's \
         surface gave: `{code}`"
    );
}
