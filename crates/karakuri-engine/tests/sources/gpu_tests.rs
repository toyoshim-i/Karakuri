//! Geometry sources, lattices, and deformation tests under mod gpu.

use crate::common::*;
use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, Set};
use karakuri_ir::typed::Checked;

mod gpu {
    use super::*;

    // ---------------------------------------------------------------------------

    /// Both sources simulate and both are drawn.
    #[test]
    fn two_sources_are_both_simulated_and_both_drawn() {
        let gpu = Gpu::headless().expect("a GPU");
        let one = lattice("one", 0.0);
        let two = lattice("two", 0.0);

        let mut single = build(&gpu, &[&one]);
        let mut pair = build(&gpu, &[&one, &two]);

        let (a, b) = (total(&gpu, &mut single), total(&gpu, &mut pair));
        assert!(a > 0.0, "the single source drew something");
        assert!(
            b > a * 1.5,
            "two sources at the same place should be brighter than one: {b} against {a}"
        );

        assert_eq!(
            pair.capacity(),
            single.capacity() * 2,
            "and a Set of two allocates both"
        );
    }

    /// Each source counts its own seed from zero.
    #[test]
    fn each_source_counts_its_own_seed_from_zero() {
        let gpu = Gpu::headless().expect("a GPU");
        let one = lattice("one", 0.0);
        let two = lattice("two", 0.0);

        let lit = |set: &mut Set| -> usize {
            frame(&gpu, set)
                .chunks_exact(4)
                .filter(|t| t[0] + t[1] + t[2] > 0.02)
                .count()
        };

        let mut single = build(&gpu, &[&one]);
        let mut pair = build(&gpu, &[&one, &two]);
        let (a, b) = (lit(&mut single), lit(&mut pair));
        assert!(a > 0, "the single source covered something");
        assert_eq!(
            a, b,
            "two lattices laid out by `seed` land on the same points, so the pair covers exactly \
         what one does — a shared counter would put the second somewhere else"
        );
    }

    /// Each source has its own hash salt.
    #[test]
    fn two_sources_of_one_procedure_differ_in_colour() {
        let gpu = Gpu::headless().expect("a GPU");
        let one = lattice("one", 0.0);
        let two = lattice("two", 0.0);

        let mut single = build(&gpu, &[&one]);
        let mut pair = build(&gpu, &[&one, &two]);

        let sum = |set: &mut Set| -> [f64; 3] {
            let px = frame(&gpu, set);
            let mut out = [0.0; 3];
            for t in px.chunks_exact(4) {
                for c in 0..3 {
                    out[c] += f64::from(t[c]);
                }
            }
            out
        };
        let a = sum(&mut single);
        let b = sum(&mut pair);

        let identical = (0..3).all(|c| (b[c] - a[c] * 2.0).abs() < a[c] * 0.02);
        assert!(
            !identical,
            "the second source drew the same colours as the first, so the salt did not move: \
         {a:?} against {b:?}"
        );
        assert!((0..3).all(|c| b[c] > a[c] * 1.2), "{a:?} against {b:?}");
    }

    /// How many pixels two frames disagree about, past what a float target rounds.
    fn disagreements(a: &[f32], b: &[f32]) -> usize {
        assert_eq!(a.len(), b.len(), "two frames of one canvas");
        a.chunks_exact(4)
            .zip(b.chunks_exact(4))
            .filter(|(x, y)| (0..3).any(|c| (x[c] - y[c]).abs() > 1e-3))
            .count()
    }

    /// A salt that was recorded travels with its geometry rather than with its position in the list.
    #[test]
    fn a_recorded_salt_survives_the_list_being_reordered() {
        let gpu = Gpu::headless().expect("a GPU");
        let near = lattice("near", 0.0);
        let far = lattice("far", 1.4);

        let mut run = build_salted(&gpu, &[&near, &far], &[]);
        let saved = frame(&gpu, &mut run);

        let recorded: Vec<Option<u32>> = run.source_salts().iter().copied().map(Some).collect();
        assert_eq!(recorded.len(), 2, "a salt per geometry, and there are two");

        let mut reloaded = build_salted(&gpu, &[&near, &far], &recorded);
        assert_eq!(
            disagreements(&saved, &frame(&gpu, &mut reloaded)),
            0,
            "a reloaded Set drew something other than the Set that was saved"
        );

        let mut swapped = build_salted(&gpu, &[&far, &near], &[recorded[1], recorded[0]]);
        assert_eq!(
            disagreements(&saved, &frame(&gpu, &mut swapped)),
            0,
            "reordering a recorded Set repainted it, so the salt is still following the \
         position rather than the geometry"
        );

        let mut derived = build_salted(&gpu, &[&far, &near], &[]);
        assert!(
            disagreements(&saved, &frame(&gpu, &mut derived)) > 16,
            "reordering an unrecorded Set drew the same picture, so this test cannot tell a \
         salt that follows the geometry from one that follows the list"
        );
    }

    /// Set::source_salts answers for every geometry, assigned or not.
    #[test]
    fn source_salts_answers_for_the_derived_ones_too() {
        let gpu = Gpu::headless().expect("a GPU");
        let one = lattice("one", 0.0);
        let two = lattice("two", 0.0);

        let both = build_salted(&gpu, &[&one, &two], &[Some(0xfeed), Some(0xbeef)]);
        assert_eq!(both.source_salts(), [0xfeed, 0xbeef]);

        let half = build_salted(&gpu, &[&one, &two], &[Some(0xfeed)]);
        assert_eq!(
            half.source_salts(),
            [0xfeed, karakuri_engine::set::derived_salt(7, 1)],
            "an unassigned source is salted from the Set's seed and its ordinal, and says so"
        );
    }

    /// A second renderer, so that "two pipelines" is two of each.
    const HALO: &str = r#"
proc halo {
  kind  L4
  blend additive

  consumes position, tint

  param exposure : float [0.0, 2.0] = 0.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.078125;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(tint * exposure, max(0.0, 1.0 - d) * 0.4);
  }
}
"#;

    const LIT: &str = r#"
proc lit {
  kind  L4
  blend additive

  consumes position, tint

  param exposure : float [0.0, 2.0] = 0.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(tint * exposure, 1.0);
  }
}
"#;

    /// Two pipelines, merged by a nested L5, published as one control.
    #[test]
    fn two_pipelines_merge_and_publish_as_one_control() {
        let gpu = Gpu::headless().expect("a GPU");
        let one_src = lattice("one", 0.0);
        let two_src = lattice("two", 0.0);

        let mut set = build_with(
            &gpu,
            &[&one_src, &two_src],
            &[LIT, HALO],
            Layering::Composite,
        );
        set.publish(karakuri_engine::set::Published {
            name: "level".to_string(),
            at: None,
            key: "exposure".to_string(),
            range: [0.0, 2.0],
        })
        .expect("both renderers declare `exposure`, so a wildcard reaches them");

        assert_eq!(set.published().len(), 1, "one control over two procedures");

        assert!(set
            .set_published("level", 2.0)
            .expect("one authority over the nodes it lands on"));
        let both = total(&gpu, &mut set);

        assert!(set.set_param_at(karakuri_ir::Kind::L4, 0, "exposure", 0.0));
        let one = total(&gpu, &mut set);
        assert!(
            one < both * 0.9,
            "silencing one renderer takes light out: {one} of {both}"
        );
        assert!(
            one > both * 0.05,
            "and leaves the other lit: {one} of {both}"
        );

        assert!(set.set_param_at(karakuri_ir::Kind::L4, 1, "exposure", 0.0));
        let none = total(&gpu, &mut set);
        assert!(
            none < both * 0.02,
            "silencing both takes all of it, so the control had reached both: {none} of {both}"
        );

        assert!(set
            .set_published("level", 2.0)
            .expect("one authority over the nodes it lands on"));
        let two_sources = total(&gpu, &mut set);
        let mut alone = build_with(&gpu, &[&one_src], &[LIT, HALO], Layering::Composite);
        alone
            .publish(karakuri_engine::set::Published {
                name: "level".to_string(),
                at: None,
                key: "exposure".to_string(),
                range: [0.0, 2.0],
            })
            .expect("the same interface");
        assert!(alone
            .set_published("level", 2.0)
            .expect("one authority over the nodes it lands on"));
        let one_source = total(&gpu, &mut alone);
        assert!(
            two_sources > one_source * 1.5,
            "two sources composited hold more light than one: {two_sources} against {one_source}"
        );
    }

    /// A pairing L2 blends two geometries element by element.
    #[test]
    fn a_pairing_l2_morphs_between_two_sources() {
        let gpu = Gpu::headless().expect("a GPU");
        let near = lattice_at("near", -1.2);
        let far = lattice_at("far", 1.2);

        let centre_x = |set: &mut Set| -> f32 {
            let px = frame(&gpu, set);
            let (mut sum, mut weight) = (0.0f64, 0.0f64);
            for (i, t) in px.chunks_exact(4).enumerate() {
                let v = f64::from(t[0] + t[1] + t[2]);
                if v > 0.02 {
                    sum += v * f64::from(i as u32 % W);
                    weight += v;
                }
            }
            assert!(weight > 0.0, "nothing was drawn");
            (sum / weight) as f32
        };

        let mut set = build_paired(&gpu, &[&near, &far], MORPH).expect("two static sources");
        let at_zero = centre_x(&mut set);

        assert!(set.set_param_at(karakuri_ir::Kind::L2, 0, "k", 1.0));
        let at_one = centre_x(&mut set);

        let mut just_near = build(&gpu, &[&near]);
        let mut just_far = build(&gpu, &[&far]);
        let (n, f) = (centre_x(&mut just_near), centre_x(&mut just_far));

        assert!(
            (at_zero - n).abs() < 1.5,
            "k=0 is the near source: {at_zero} against {n}"
        );
        assert!(
            (at_one - f).abs() < 1.5,
            "k=1 is the paired source: {at_one} against {f}"
        );
        assert!(
            (n - f).abs() > 8.0,
            "the two sources are far enough apart to tell apart"
        );

        assert!(set.set_param_at(karakuri_ir::Kind::L2, 0, "k", 0.5));
        let half = centre_x(&mut set);
        assert!(
            (half - (n + f) / 2.0).abs() < 2.0,
            "k=0.5 sits between the two: {half} against {}",
            (n + f) / 2.0
        );
    }

    /// The paired geometry is not drawn.
    #[test]
    fn the_paired_geometry_is_never_drawn_on_its_own() {
        let gpu = Gpu::headless().expect("a GPU");
        let near = lattice_at("near", -1.2);
        let far = lattice_at("far", 1.2);

        let mut paired = build_paired(&gpu, &[&near, &far], MORPH).expect("two static sources");
        let mut alone = build(&gpu, &[&near]);

        let (a, b) = (total(&gpu, &mut paired), total(&gpu, &mut alone));
        assert!(
            (a - b).abs() < b * 0.05,
            "a morph at k=0 holds one lattice's worth of light, not two: {a} against {b}"
        );
        assert_eq!(
            paired.capacity(),
            alone.capacity(),
            "and allocates one lattice to be drawn"
        );
    }

    /// The edge decides which geometry the slot is bound to, and the list's order decides nothing.
    #[test]
    fn the_edge_says_which_geometry_is_bound_and_the_list_order_does_not() {
        let gpu = Gpu::headless().expect("a GPU");
        let near = lattice_at("near", -1.2);
        let far = lattice_at("far", 1.2);

        let centre_x = |set: &mut Set| -> f32 {
            let px = frame(&gpu, set);
            let (mut sum, mut weight) = (0.0f64, 0.0f64);
            for (i, t) in px.chunks_exact(4).enumerate() {
                let v = f64::from(t[0] + t[1] + t[2]);
                if v > 0.02 {
                    sum += v * f64::from(i as u32 % W);
                    weight += v;
                }
            }
            assert!(weight > 0.0, "nothing was drawn");
            (sum / weight) as f32
        };
        let at_one = |l1s: &[&str]| -> f32 {
            let mut set = build_wired(&gpu, l1s, MORPH, &[edge("morph", "far", "far")])
                .expect("two static sources and a bound slot");
            assert!(set.set_param_at(karakuri_ir::Kind::L2, 0, "k", 1.0));
            centre_x(&mut set)
        };

        let listed_near_first = at_one(&[&near, &far]);
        let listed_far_first = at_one(&[&far, &near]);
        assert!(
            (listed_near_first - listed_far_first).abs() < 1.5,
            "the same edge over the same two sources is the same picture whichever \
         order they were listed in: {listed_near_first} against {listed_far_first}"
        );

        let mut other_way =
            build_wired(&gpu, &[&near, &far], MORPH, &[edge("morph", "far", "near")])
                .expect("the near lattice is a geometry like any other");
        assert!(other_way.set_param_at(karakuri_ir::Kind::L2, 0, "k", 1.0));
        let bound_to_near = centre_x(&mut other_way);
        assert!(
            (bound_to_near - listed_near_first).abs() > 8.0,
            "binding the slot to the other geometry is a different picture: \
         {bound_to_near} against {listed_near_first}"
        );
    }

    /// An edge naming a node this Set has not got is about another Set.
    #[test]
    fn an_edge_about_another_set_is_passed_over() {
        let gpu = Gpu::headless().expect("a GPU");
        let near = lattice_at("near", -1.2);
        let far = lattice_at("far", 1.2);

        let built = build_wired(
            &gpu,
            &[&near, &far],
            MORPH,
            &[
                edge("someone_elses_node", "far", "near"),
                edge("morph", "far", "far"),
            ],
        );
        assert!(
            built.is_ok(),
            "an edge about a node this Set has not got is not this Set's business: {:?}",
            built.err()
        );

        let err = build_wired(&gpu, &[&near, &far], MORPH, &[edge("morhp", "far", "far")])
            .err()
            .expect("nothing bound the slot");
        assert!(
            err.to_string()
                .contains("`far : Geometry` and nothing in this Set says what fills it"),
            "{err}"
        );
    }

    /// The paired geometry's own params reach it.
    #[test]
    fn a_paired_sources_own_params_reach_it() {
        let gpu = Gpu::headless().expect("a GPU");
        let near = lattice_at("near", -1.2);
        let far = lattice_at("far", 1.2).replace(
            "  emit position, tint",
            "  param push : float [0.0, 2.0] = 0.0\n\n  emit position, tint",
        );
        let far = far.replace("* 0.5 - 1.75, 1.2)", "* 0.5 - 1.75, 1.2 + push)");

        let centre_x = |set: &mut Set| -> f32 {
            let px = frame(&gpu, set);
            let (mut sum, mut weight) = (0.0f64, 0.0f64);
            for (i, t) in px.chunks_exact(4).enumerate() {
                let v = f64::from(t[0] + t[1] + t[2]);
                if v > 0.02 {
                    sum += v * f64::from(i as u32 % W);
                    weight += v;
                }
            }
            assert!(weight > 0.0, "nothing was drawn");
            (sum / weight) as f32
        };

        let mut set = build_paired(&gpu, &[&near, &far], MORPH).expect("two static sources");
        assert!(set.set_param_at(karakuri_ir::Kind::L2, 0, "k", 1.0));
        let before = centre_x(&mut set);

        assert!(
            set.set_param_at(karakuri_ir::Kind::L1, 1, "push", 2.0),
            "the far geometry is an L1 procedure and `L1:1` addresses it"
        );
        let after = centre_x(&mut set);
        assert!(
            (after - before).abs() > 3.0,
            "the far source's own param has to reach its own shader: {before} against {after}"
        );

        let mut reversed = build_wired(&gpu, &[&far, &near], MORPH, &[edge("morph", "far", "far")])
            .expect("two static sources, listed the other way round");
        assert!(reversed.set_param_at(karakuri_ir::Kind::L2, 0, "k", 1.0));
        let before = centre_x(&mut reversed);
        assert!(
            reversed.set_param_at(karakuri_ir::Kind::L1, 0, "push", 2.0),
            "`far` is the first L1 procedure here, so `L1:0` is the one declaring `push`"
        );
        let after = centre_x(&mut reversed);
        assert!(
            (after - before).abs() > 3.0,
            "the far source's param reached the wrong shader: {before} against {after}"
        );
    }

    /// Both sides of a pairing carry the same element struct.
    #[test]
    fn both_sides_of_a_pairing_share_the_element_struct() {
        let gpu = Gpu::headless().expect("a GPU");
        let near = lattice_at("near", -1.2);
        let far = lattice_at("far", 1.2);

        let aged = DOTS
            .replace("consumes position, tint", "consumes position, tint, age")
            .replace(
                "    color = vec4(tint, 1.0);",
                "    color = vec4(tint, 1.0) * (1.0 + age * 0.0);",
            );

        let compiled: Vec<Checked> = [&near, &far].iter().map(|s| compile(s)).collect();
        let sources: Vec<(&Checked, u32)> = compiled.iter().map(|c| (c, 64)).collect();
        let l2 = compile(MORPH);
        let l4 = compile(&aged);
        let mut set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &sources,
            &[&l2],
            &[],
            &[],
            &[&l4],
            Layering::Overdraw,
            7,
            &[],
            karakuri_engine::set::Wiring {
                edges: &[edge("morph", "far", "far")],
                ..Default::default()
            },
        )
        .expect("two static sources and a derived attribute");
        set.resize(&gpu.device, W, H);
        set.aim_camera(karakuri_engine::camera::Orbit {
            radius: 6.0,
            speed: 0.0,
            height: 0.0,
            ..Default::default()
        });

        let centre_x = |set: &mut Set| -> f32 {
            let px = frame(&gpu, set);
            let (mut sum, mut weight) = (0.0f64, 0.0f64);
            for (i, t) in px.chunks_exact(4).enumerate() {
                let v = f64::from(t[0] + t[1] + t[2]);
                if v > 0.02 {
                    sum += v * f64::from(i as u32 % W);
                    weight += v;
                }
            }
            assert!(weight > 0.0, "nothing was drawn");
            (sum / weight) as f32
        };

        assert!(set.set_param_at(karakuri_ir::Kind::L2, 0, "k", 1.0));
        let at_one = centre_x(&mut set);

        let mut just_far = build_with(&gpu, &[&far], &[&aged], Layering::Overdraw);
        let f = centre_x(&mut just_far);
        assert!(
            (at_one - f).abs() < 1.5,
            "k=1 has to land on the far lattice: {at_one} against {f}"
        );
    }

    /// Every node of a Set has a name, and a name resolves to the node.
    #[test]
    fn a_name_resolves_to_the_node_it_addresses() {
        let gpu = Gpu::headless().expect("a GPU");
        let near = compile(&lattice("near_grid", 0.0));
        let far = compile(&lattice("far_grid", 0.5));
        let draw = compile(DOTS);

        let set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&near, 64), (&far, 64)],
            &[],
            &[],
            &[],
            &[&draw],
            Layering::Overdraw,
            0,
            &[],
            karakuri_engine::set::Wiring {
                l1s: &[Some("near".to_string()), None],
                ..Default::default()
            },
        )
        .expect("builds");

        assert_eq!(
            set.node_names(),
            ["near", "far_grid", "orbit", "dots"].map(String::from)
        );
        assert_eq!(set.node_named("near"), Some((karakuri_ir::Kind::L1, 0)));
        assert_eq!(set.node_named("far_grid"), Some((karakuri_ir::Kind::L1, 1)));
        assert_eq!(set.node_named("dots"), Some((karakuri_ir::Kind::L4, 0)));
        assert_eq!(set.node_named("nothing_is_called_this"), None);
    }

    /// Two uses of one procedure are two nodes with distinct derived names.
    #[test]
    fn a_procedure_used_twice_gives_its_second_node_a_different_name() {
        let gpu = Gpu::headless().expect("a GPU");
        let grid = compile(&lattice("grid", 0.0));
        let draw = compile(DOTS);

        let set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&grid, 64), (&grid, 64)],
            &[],
            &[],
            &[],
            &[&draw, &draw],
            Layering::Overdraw,
            0,
            &[],
            karakuri_engine::set::Wiring::default(),
        )
        .expect("builds");

        assert_eq!(
            set.node_names(),
            ["grid", "grid-2", "orbit", "dots", "dots-2"].map(String::from)
        );
        assert_eq!(set.node_named("grid-2"), Some((karakuri_ir::Kind::L1, 1)));
        assert_eq!(set.node_named("dots-2"), Some((karakuri_ir::Kind::L4, 1)));
    }

    /// A written name is not stolen by a derived one that reaches it first.
    #[test]
    fn a_derived_name_never_takes_one_that_was_written() {
        let gpu = Gpu::headless().expect("a GPU");
        let grid = compile(&lattice("grid", 0.0));
        let draw = compile(DOTS);

        let set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &[(&grid, 64)],
            &[],
            &[],
            &[],
            &[&draw, &draw],
            Layering::Overdraw,
            0,
            &[],
            karakuri_engine::set::Wiring {
                l4s: &[None, Some("dots".to_string())],
                ..Default::default()
            },
        )
        .expect("builds");

        assert_eq!(set.node_named("dots"), Some((karakuri_ir::Kind::L4, 1)));
        assert_eq!(set.node_names()[2], "dots-2");
    }

    // ---------------------------------------------------------------------------
    // Masks and dissolve tests
    // ---------------------------------------------------------------------------

    /// The same lattice with the salt taken out of its colour.
    fn white_lattice(name: &str, z: f32) -> String {
        let src = lattice_at(name, z);
        let tint = "tint     = vec3(hash1(seed), hash1(seed + 1u), hash1(seed + 2u));";
        assert!(src.contains(tint), "the tint line moved: {src}");
        src.replace(tint, "tint     = vec3(1.0, 1.0, 1.0);")
    }

    /// The horizontal centre of the light in the frame.
    fn lit_centre(gpu: &Gpu, set: &mut Set) -> f32 {
        let px = frame(gpu, set);
        let (mut sum, mut weight) = (0.0f64, 0.0f64);
        for (i, t) in px.chunks_exact(4).enumerate() {
            let v = f64::from(t[0] + t[1] + t[2]);
            if v > 0.02 {
                sum += v * f64::from(i as u32 % W);
                weight += v;
            }
        }
        assert!(weight > 0.0, "nothing was drawn");
        (sum / weight) as f32
    }

    /// A mask dissolves one of two sources and leaves the other standing.
    #[test]
    fn a_mask_dissolves_the_source_its_slot_names() {
        let gpu = Gpu::headless().expect("a GPU");
        let left = white_lattice("left", -1.2);
        let right = white_lattice("right", 1.2);

        let (mut only_left, mut only_right) = (build(&gpu, &[&left]), build(&gpu, &[&right]));
        let (at_left, at_right) = (
            lit_centre(&gpu, &mut only_left),
            lit_centre(&gpu, &mut only_right),
        );
        let one_lattice = total(&gpu, &mut only_left);
        assert!(
            (at_left - at_right).abs() > 8.0,
            "the two sit far enough apart to tell apart: {at_left} against {at_right}"
        );

        let mut both = build_with(&gpu, &[&left, &right], &[DOTS], Layering::Overdraw);
        let two_lattices = total(&gpu, &mut both);

        for (bound, survivor, gone) in [("right", at_left, at_right), ("left", at_right, at_left)] {
            let mut set = build_wired(
                &gpu,
                &[&left, &right],
                DISSOLVE,
                &[edge("dissolve", "only", bound)],
            )
            .expect("a Set whose mask names one of its two sources");

            let light = total(&gpu, &mut set);
            assert!(
                (light - one_lattice).abs() < one_lattice * 0.15,
                "binding `only` to `{bound}` leaves one lattice's worth of light: \
             {light} against {one_lattice} (both would be {two_lattices})"
            );

            let centre = lit_centre(&gpu, &mut set);
            assert!(
                (centre - survivor).abs() < 2.0,
                "and it is `{bound}` that went, not the other: the light is at {centre}, \
             the survivor is at {survivor} and the dissolved one was at {gone}"
            );
        }
    }

    /// Two Source slots on one node are two independent edges.
    #[test]
    fn two_source_slots_on_one_node_name_two_geometries() {
        let gpu = Gpu::headless().expect("a GPU");
        let a = white_lattice("a", -1.4);
        let b = white_lattice("b", 0.0);
        let c = white_lattice("c", 1.4);

        let pair = r#"
proc dissolve {
  kind L2

  uses one : Source
  uses two : Source

  consumes position, tint

  mask {
    strength = 0.0;
    if source == one || source == two { strength = 1.0; }
  }

  deform {
    tint = vec3(0.0, 0.0, 0.0);
  }
}
"#;

        for spared in ["a", "b", "c"] {
            let bound: Vec<&str> = ["a", "b", "c"]
                .into_iter()
                .filter(|n| *n != spared)
                .collect();
            let mut set = build_wired(
                &gpu,
                &[&a, &b, &c],
                pair,
                &[
                    edge("dissolve", "one", bound[0]),
                    edge("dissolve", "two", bound[1]),
                ],
            )
            .expect("a Set whose mask names two of its three sources");

            let mut alone = build(
                &gpu,
                &[match spared {
                    "a" => &a,
                    "b" => &b,
                    _ => &c,
                }],
            );
            let (masked, expected) = (total(&gpu, &mut set), total(&gpu, &mut alone));
            assert!(
                (masked - expected).abs() < expected * 0.15,
                "sparing `{spared}` leaves exactly that lattice: {masked} against {expected}"
            );
            assert!(
                (lit_centre(&gpu, &mut set) - lit_centre(&gpu, &mut alone)).abs() < 2.0,
                "and it is `{spared}` that is left standing"
            );
        }
    }
}
