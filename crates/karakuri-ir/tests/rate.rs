//! Integration tests verifying static point rate bounds against example `.kir` files.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use karakuri_ir::rate::{point_rate_bound, Bound};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

fn kir_files() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "kir"))
        .collect();
    found.sort();
    found
}

/// Every renderer in `examples/`, by procedure name.
fn renderers() -> BTreeMap<String, Checked> {
    let mut found = BTreeMap::new();
    for path in kir_files() {
        let src = std::fs::read_to_string(&path).expect("read");
        let proc = karakuri_ir::parse(&src)
            .unwrap_or_else(|e| panic!("{} does not parse: {e:?}", path.display()));
        let checked = karakuri_ir::check::check(&proc)
            .unwrap_or_else(|e| panic!("{} does not check: {e:?}", path.display()));
        if checked.kind == Kind::L4 {
            found.insert(checked.name.clone(), checked);
        }
    }
    assert!(!found.is_empty(), "no renderers found in examples/");
    found
}

/// Verifies rate bounding on every shipped renderer example.
#[test]
fn every_shipped_renderer_is_bounded_or_named_as_refused() {
    // (procedure, rate bounded to, floor in rows)
    const BOUNDED: &[(&str, f32, u32)] = &[
        // `point_scale`'s declared minimum, which *is* the rate.
        ("plain_points", 0.0014, 715),
        // The same, times `max(size, 1.0)`.
        ("star_flares", 0.0014, 715),
        // The constant low end of the procedure's own `clamp`.
        ("sheet_shade", 0.00139, 720),
        // `width`'s declared minimum, which is the rate.
        ("speed_lines", 0.00069, 1450),
        // `point_scale`'s minimum times the 0.35 an element at rest gets.
        ("soft_points", 0.000_241_5, 4141),
        ("second_eye", 0.000_241_5, 4141),
        ("glass_shell", 0.000_241_5, 4141),
        // The same shape on `width`.
        ("drift_streaks", 0.000_241_5, 4141),
        // Constant low end of clamp(w, 0.001, 0.06).
        ("audio_bloom", 0.001, 1000),
    ];

    /// A renderer with no `vertex` block, which emits no rate at all.
    const FULLSCREEN: &[&str] = &["field_lens", "field_march", "glow_march"];

    /// **What is refused, and why**, which is the half of this that tells a
    /// maintainer whether the analysis is worth extending. Two of these are
    /// attributes with no declaration to read; two are rates that genuinely
    /// reach zero, where no height makes the primitive a pixel.
    const REFUSED: &[(&str, f32)] = &[
        // `dot_scale * size` — `size` is an attribute.
        ("hard_dots", f32::NEG_INFINITY),
        // `... * (1.0 + age)` — `age` is an attribute.
        ("beat_strokes", f32::NEG_INFINITY),
        // `width * max(spill, age * glitch_glow)`, and `spill` reaches zero.
        ("beat_bloom", 0.0),
        // `width_var` is declared up to 1.0, where the rate is `hash1` times
        // zero.
        ("strand_strokes", 0.0),
    ];

    let renderers = renderers();
    let mut seen = Vec::new();

    for (name, rate, floor) in BOUNDED {
        let checked = renderers
            .get(*name)
            .unwrap_or_else(|| panic!("examples/ no longer ships `{name}`"));
        match point_rate_bound(checked).bound {
            Bound::AtLeast { rate: got, .. } => {
                assert!(
                    (got - rate).abs() <= rate * 1e-4,
                    "`{name}` bounds to {got} where its declarations imply {rate}"
                );
                assert_eq!(
                    (1.0 / f64::from(got)).ceil() as u32,
                    *floor,
                    "`{name}`'s floor moved"
                );
            }
            other => panic!("`{name}` used to bound and now answers {other:?}"),
        }
        seen.push(*name);
    }

    for name in FULLSCREEN {
        let checked = renderers
            .get(*name)
            .unwrap_or_else(|| panic!("examples/ no longer ships `{name}`"));
        assert_eq!(
            point_rate_bound(checked).bound,
            Bound::NoPrimitive,
            "`{name}` draws no primitive"
        );
        seen.push(*name);
    }

    for (name, lower) in REFUSED {
        let checked = renderers
            .get(*name)
            .unwrap_or_else(|| panic!("examples/ no longer ships `{name}`"));
        match point_rate_bound(checked).bound {
            Bound::Unbounded { lower: got, .. } => assert_eq!(
                got, *lower,
                "`{name}` is still refused but for a different reason"
            ),
            other => panic!(
                "`{name}` is now bounded, which is good news and this file's \
                 count is stale: {other:?}"
            ),
        }
        seen.push(*name);
    }

    // **Nothing unaccounted for.** A renderer added to `examples/` and not to
    // one of the three lists above is a coverage figure nobody re-took.
    let mut missing: Vec<&str> = renderers
        .keys()
        .map(String::as_str)
        .filter(|n| !seen.contains(n))
        .collect();
    missing.sort();
    assert!(
        missing.is_empty(),
        "these renderers are in examples/ and in none of the three lists: {missing:?}"
    );
}

/// **The coverage figure, stated as a figure**, because *what fraction does it
/// answer for* is the question this analysis was built to answer and a reader
/// should not have to count the list above.
#[test]
fn nine_of_the_thirteen_renderers_that_draw_a_primitive_state_a_floor() {
    let renderers = renderers();
    let mut fullscreen = 0;
    let mut bounded = 0;
    let mut refused = 0;
    for checked in renderers.values() {
        match point_rate_bound(checked).bound {
            Bound::NoPrimitive => fullscreen += 1,
            Bound::AtLeast { .. } => bounded += 1,
            Bound::Unbounded { .. } => refused += 1,
        }
    }
    assert_eq!(
        (fullscreen, bounded, refused),
        (3, 9, 4),
        "the shipped corpus's coverage moved; the lists in this file say which way"
    );
}
