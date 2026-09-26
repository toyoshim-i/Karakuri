//! Measurement scaffolding benchmarking execution cost against IR operation ceilings.
//!
//! Run with: `cargo run -p karakuri-engine --example ceiling_ladder --release`

use karakuri_engine::probe::{Measurement, MeasurementMethod};
use karakuri_engine::set::{Edge, Layering, Wiring, MAX_STEPS};
use karakuri_engine::{Gpu, Probe, Set, Signals, VideoSource};
use karakuri_ir::typed::{Checked, Cost};

mod procedures;
use procedures::*;

/// Target resolution used for ladder benchmark measurements.
const AT: (u32, u32) = (1280, 720);

const CAPACITY: u32 = 262_144;
const SEED: u32 = 19_274;

/// The shipped values, hardcoded because the real constants are patched for
/// the duration of the run.
const SHIPPED_MAX_OPS_PER_ELEMENT: u64 = 4096;
const SHIPPED_MAX_OPS_PER_SPAWN: u64 = 16_384;
const SHIPPED_MAX_OPS_PER_FRAGMENT: u64 = 512;
const SHIPPED_MAX_OPS_PER_FULLSCREEN_FRAGMENT: u64 = SHIPPED_MAX_OPS_PER_ELEMENT;

fn compile_str(what: &str, src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{what}: parse: {e:?}"));
    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{what}: check: {e:?}"))
}

fn compile_file(path: &str) -> Checked {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    compile_str(path, &src)
}

fn cost_of(what: &str, c: &Checked) -> Cost {
    karakuri_ir::cost::estimate(c).unwrap_or_else(|e| {
        panic!(
            "{what}: cost::estimate refused — the ceilings in cost.rs were NOT patched, \
             so this run would have measured only the rungs that already fit: {e:?}"
        )
    })
}

/// The caller's own fragment estimate with each field slot's per-evaluation
/// cost multiplied in — the figure `check_with_field` compares to the ceiling.
fn fragment_ops_with_fields(caller: &Cost, per_eval: &dyn Fn(&str) -> u64) -> u64 {
    caller
        .field_calls
        .slots
        .iter()
        .fold(caller.ops_per_fragment, |sum, s| {
            sum + s.per_fragment * per_eval(&s.slot)
        })
}

struct Rig {
    gpu: Gpu,
    probe: Probe,
    warm_target: wgpu::TextureView,
}

impl Rig {
    fn new() -> Rig {
        let gpu = Gpu::headless().expect("no GPU available");
        // Single probe shared across process to ensure consistent sample timing.
        let probe = Probe::new(&gpu.device, &gpu.queue, true, AT);
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ceiling ladder warm target"),
            size: wgpu::Extent3d {
                width: AT.0,
                height: AT.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        Rig {
            gpu,
            probe,
            warm_target: target.create_view(&Default::default()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn measure(
        &mut self,
        what: &str,
        l1: &Checked,
        fields: &[&Checked],
        l4: &Checked,
        edges: &[Edge],
        warm_steps: u32,
        params: &[(&str, f32)],
    ) -> Measurement {
        let mut set = Set::build_many(
            &self.gpu.device,
            &self.gpu.queue,
            &[(l1, CAPACITY)],
            &[],
            &[],
            fields,
            &[l4],
            Layering::Overdraw,
            SEED,
            &[],
            Wiring {
                edges,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("{what}: Set::build_many: {e}"));
        set.resize(&self.gpu.device, AT.0, AT.1);
        for (name, value) in params {
            let n = set.set_param(name, *value);
            assert!(n > 0, "{what}: no node declares param `{name}`");
        }

        let mut remaining = warm_steps;
        while remaining > 0 {
            let steps = remaining.min(u32::from(MAX_STEPS)) as u8;
            set.prepare(&self.gpu.queue, steps, &Signals::default());
            let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
            set.render(&mut encoder, &self.warm_target, steps);
            self.gpu.queue.submit([encoder.finish()]);
            remaining -= u32::from(steps);
        }

        set.prepare(&self.gpu.queue, 1, &Signals::default());
        self.probe
            .run(&self.gpu.device, &self.gpu.queue, &mut set, 1, CAPACITY)
    }
}

struct Rung {
    label: String,
    dial: String,
    ops: u64,
    ms: f32,
    lo: f32,
    hi: f32,
}

/// Ladder benchmark configuration and procedure metadata for interleaved passes.
struct Spec {
    label: String,
    dial: String,
    ops: u64,
    l1: Checked,
    fields: Vec<Checked>,
    l4: Checked,
    edges: Vec<Edge>,
    warm: u32,
    params: Vec<(String, f32)>,
}

/// Passes over the whole ladder. The first is discarded: a process's very
/// first measurement runs on a cold device and reads low, which is the
/// opposite of the heating above and just as much an artifact.
const PASSES: u32 = 3;

fn run_ladder(rig: &mut Rig, specs: &[Spec]) -> (Vec<Rung>, Measurement) {
    let mut samples: Vec<Vec<f32>> = vec![Vec::new(); specs.len()];
    let mut last = None;
    for pass in 0..PASSES {
        for (i, spec) in specs.iter().enumerate() {
            let fields: Vec<&Checked> = spec.fields.iter().collect();
            let params: Vec<(&str, f32)> =
                spec.params.iter().map(|(n, v)| (n.as_str(), *v)).collect();
            let m = rig.measure(
                &spec.label,
                &spec.l1,
                &fields,
                &spec.l4,
                &spec.edges,
                spec.warm,
                &params,
            );
            eprintln!(
                "  pass {pass} | {} | {} | {} ops | {:.3} ms",
                spec.label, spec.dial, spec.ops, m.ms
            );
            if pass > 0 {
                samples[i].push(m.ms);
            }
            last = Some(m);
        }
    }
    let rungs = specs
        .iter()
        .zip(samples)
        .map(|(spec, mut s)| {
            s.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            Rung {
                label: spec.label.clone(),
                dial: spec.dial.clone(),
                ops: spec.ops,
                ms: s[s.len() / 2],
                lo: s[0],
                hi: s[s.len() - 1],
            }
        })
        .collect();
    (rungs, last.expect("at least one rung"))
}

fn print_table(title: &str, note: &str, ceiling: u64, axis: &str, rungs: &[Rung], m: &Measurement) {
    println!("\n## {title}");
    println!("{note}");
    println!(
        "shipped ceiling: {ceiling} {axis} | method: {:?} | capacity: {} | resolution: {}x{}",
        m.method, m.capacity, m.resolution.0, m.resolution.1
    );
    println!(
        "| procedure + field | dial | {axis} | ms (median of {}) | spread | over ceiling |",
        PASSES - 1
    );
    println!("|---|---|---|---|---|---|");
    for r in rungs {
        println!(
            "| {} | {} | {} | {:.3} | {:.3}-{:.3} | {} |",
            r.label,
            r.dial,
            r.ops,
            r.ms,
            r.lo,
            r.hi,
            if r.ops > ceiling { "yes" } else { "" }
        );
    }
}

/// A ladder whose over-ceiling rungs silently failed to build reads as a flat
/// cheap line. This is the guard the design asks for.
fn guard(title: &str, ceiling: u64, rungs: &[Rung]) {
    for w in rungs.windows(2) {
        assert!(
            w[1].ops > w[0].ops,
            "{title}: estimates are not strictly increasing ({} then {}) — \
             the ladder is not a ladder",
            w[0].ops,
            w[1].ops
        );
    }
    assert!(
        rungs.iter().any(|r| r.ops > ceiling),
        "{title}: no rung exceeds the {ceiling} ceiling — the constants in cost.rs \
         were not patched, or the ladder is too short to say anything"
    );
}

// ---------------------------------------------------------------- ladder 1

/// The estimated ops/fragment `field_lens` at `steps` reaches with `field`
/// spliced in — computed the way `Set::build` computes it.
fn lens_ops(steps: u32, field_eval: u64) -> u64 {
    let lens = compile_str("field_lens", &field_lens_src(steps));
    let c = cost_of("field_lens", &lens);
    fragment_ops_with_fields(&c, &|_| field_eval)
}

/// The step count at which `field` lands closest to `target` ops/fragment.
fn steps_matching(target: u64, field_eval: u64, max_steps: u32) -> u32 {
    (1..=max_steps)
        .min_by_key(|s| lens_ops(*s, field_eval).abs_diff(target))
        .expect("at least one step count")
}

fn ladder_fullscreen(rig: &mut Rig) {
    let l1 = compile_file("examples/drift_shell.kir");
    let melt = compile_file("examples/melt_blob.kir");
    let beam = compile_file("examples/beam_lattice.kir");
    let melt_eval = cost_of("melt_blob", &melt).ops_per_evaluation;
    let beam_eval = cost_of("beam_lattice", &beam).ops_per_evaluation;
    println!("\nmelt_blob = {melt_eval} ops/evaluation, beam_lattice = {beam_eval} ops/evaluation");

    let steps_list = [8u32, 16, 26, 34, 40, 48, 64, 96, 128, 192, 256];
    let mut melt_specs = Vec::new();
    for steps in steps_list {
        let lens = compile_str("field_lens", &field_lens_src(steps));
        let ops = fragment_ops_with_fields(&cost_of("field_lens", &lens), &|_| melt_eval);
        melt_specs.push(Spec {
            label: "field_lens + melt_blob".into(),
            dial: format!("{steps} steps"),
            ops,
            l1: l1.clone(),
            fields: vec![melt.clone()],
            l4: lens,
            edges: vec![Edge {
                node: "field_lens".into(),
                slot: "shape".into(),
                to: "melt_blob".into(),
            }],
            warm: 0,
            params: vec![],
        });
    }
    let (melt_rungs, m) = run_ladder(rig, &melt_specs);
    guard(
        "fullscreen / melt_blob",
        SHIPPED_MAX_OPS_PER_FULLSCREEN_FRAGMENT,
        &melt_rungs,
    );
    print_table(
        "Ladder 1a - fullscreen fragment, field_lens + melt_blob",
        "dial is the march step count; 34 ships, 40 is refused today",
        SHIPPED_MAX_OPS_PER_FULLSCREEN_FRAGMENT,
        "ops/fragment",
        &melt_rungs,
        &m,
    );

    // Same ladder, second material, at step counts chosen to land on the same
    // ops/fragment figures. This is the check on whether ops/fragment is the
    // right x-axis at all.
    let mut beam_specs: Vec<Spec> = Vec::new();
    for r in &melt_rungs {
        let steps = steps_matching(r.ops, beam_eval, 256);
        let lens = compile_str("field_lens", &field_lens_src(steps));
        let ops = fragment_ops_with_fields(&cost_of("field_lens", &lens), &|_| beam_eval);
        if beam_specs.last().map(|p| p.ops >= ops).unwrap_or(false) {
            continue; // two melt rungs matched the same beam step count
        }
        beam_specs.push(Spec {
            label: "field_lens + beam_lattice".into(),
            dial: format!("{steps} steps (matched to {} ops)", r.ops),
            ops,
            l1: l1.clone(),
            fields: vec![beam.clone()],
            l4: lens,
            edges: vec![Edge {
                node: "field_lens".into(),
                slot: "shape".into(),
                to: "beam_lattice".into(),
            }],
            warm: 0,
            params: vec![],
        });
    }
    let (beam_rungs, m) = run_ladder(rig, &beam_specs);
    guard(
        "fullscreen / beam_lattice",
        SHIPPED_MAX_OPS_PER_FULLSCREEN_FRAGMENT,
        &beam_rungs,
    );
    print_table(
        "Ladder 1b - same ladder, beam_lattice, matched estimates",
        "if two procedures with equal estimated ops/fragment measure >2x apart, \
         the weights are mispriced rather than the ceiling",
        SHIPPED_MAX_OPS_PER_FULLSCREEN_FRAGMENT,
        "ops/fragment",
        &beam_rungs,
        &m,
    );
    println!("\nmatched-estimate ratio (beam ms / melt ms), the >2x test:");
    for b in &beam_rungs {
        if let Some(mel) = melt_rungs.iter().min_by_key(|r| r.ops.abs_diff(b.ops)) {
            println!(
                "- {} ops melt {:.3} ms vs {} ops beam {:.3} ms -> {:.2}x",
                mel.ops,
                mel.ms,
                b.ops,
                b.ms,
                b.ms / mel.ms
            );
        }
    }
}

// -------------------------------------------------- ladder 1c: the normal

fn ladder_field_march(rig: &mut Rig) {
    let l1 = compile_file("examples/drift_shell.kir");
    let mut specs = Vec::new();
    for correct in [false, true] {
        let march = compile_str("field_march", &field_march_src(correct));
        let ops = cost_of("field_march", &march).ops_per_fragment;
        specs.push(Spec {
            label: "field_march (inline field)".into(),
            dial: if correct {
                "48 steps + six-tap normal".into()
            } else {
                "48 steps, sphere-only normal (ships)".into()
            },
            ops,
            l1: l1.clone(),
            fields: vec![],
            l4: march,
            edges: vec![],
            warm: 0,
            params: vec![],
        });
    }
    let (rungs, m) = run_ladder(rig, &specs);
    guard(
        "field_march normal",
        SHIPPED_MAX_OPS_PER_FULLSCREEN_FRAGMENT,
        &rungs,
    );
    print_table(
        "Ladder 1c - the picture the ceiling is costing, priced",
        "the correct surface normal `field_march.kir` says does not fit",
        SHIPPED_MAX_OPS_PER_FULLSCREEN_FRAGMENT,
        "ops/fragment",
        &rungs,
        &m,
    );
}

// ---------------------------------------------------------------- ladder 2

fn ladder_element_fragment(rig: &mut Rig) {
    let l1 = compile_file("examples/drift_shell.kir");
    let shipped = compile_file("examples/soft_points.kir");
    let sc = cost_of("soft_points", &shipped);
    println!(
        "\nsoft_points as shipped: {} ops/fragment, {} ops/element(vertex)",
        sc.ops_per_fragment, sc.ops_per_element
    );

    // Ladder A: ops at fixed coverage.
    let mut specs: Vec<Spec> = Vec::new();
    for pad in [0u32, 4, 12, 30, 65, 135, 275, 555, 1115] {
        let l4 = compile_str("pad_points", &pad_points_src(pad));
        let ops = cost_of("pad_points", &l4).ops_per_fragment;
        if specs.iter().any(|p| p.ops == ops) {
            continue;
        }
        specs.push(Spec {
            label: "drift_shell + pad_points".into(),
            dial: format!("pad {pad}"),
            ops,
            l1: l1.clone(),
            fields: vec![],
            l4,
            edges: vec![],
            warm: 0,
            params: vec![],
        });
    }
    // The zero-coverage control: the same procedure at a half-pixel sprite, so
    // what is left is the simulation, the draw and the host clock's own
    // overhead — the constant that has to be subtracted before any figure here
    // is about fragments at all.
    let pad0 = compile_str("pad_points", &pad_points_src(0));
    specs.insert(
        0,
        Spec {
            label: "drift_shell + pad_points".into(),
            dial: "pad 0, 0.5 px sprite (coverage control)".into(),
            ops: 0,
            l1: l1.clone(),
            fields: vec![],
            l4: pad0.clone(),
            edges: vec![],
            warm: 0,
            params: vec![("point_scale".to_string(), 0.00069)],
        },
    );
    let (rungs, m) = run_ladder(rig, &specs);
    guard("per-element fragment", SHIPPED_MAX_OPS_PER_FRAGMENT, &rungs);
    print_table(
        "Ladder 2A - per-element fragment: ops at fixed coverage",
        "point_rate fixed at the shipped default 0.00556 (a 4.0 px sprite at 720p), so \
         coverage is capacity * (0.00556*720)^2 = 4.2M fragments, 4.6x the canvas",
        SHIPPED_MAX_OPS_PER_FRAGMENT,
        "ops/fragment",
        &rungs,
        &m,
    );

    // Ladder B: coverage at fixed ops.
    let ops = cost_of("pad_points", &pad0).ops_per_fragment;
    let scales = [
        0.00069f32, 0.00139, 0.00278, 0.00556, 0.0111, 0.0222, 0.0556,
    ];
    let mut specs: Vec<Spec> = Vec::new();
    for scale in scales {
        let side = scale * AT.1 as f32;
        specs.push(Spec {
            label: "drift_shell + pad_points".into(),
            dial: format!("point_scale {scale} = {side:.2} px"),
            ops,
            l1: l1.clone(),
            fields: vec![],
            l4: pad0.clone(),
            edges: vec![],
            warm: 0,
            params: vec![("point_scale".to_string(), scale)],
        });
    }
    let (rungs, m) = run_ladder(rig, &specs);
    println!("\n## Ladder 2B - per-element fragment: coverage at fixed ops");
    println!("pad_points at {ops} ops/fragment throughout; the dial is point_scale");
    println!(
        "method: {:?} | capacity: {} | resolution: {}x{}",
        m.method, m.capacity, m.resolution.0, m.resolution.1
    );
    println!("| point_scale | sprite side (px) | fragments (M) | canvases | ms | spread |");
    println!("|---|---|---|---|---|---|");
    let pixels = f64::from(AT.0) * f64::from(AT.1);
    for (scale, r) in scales.iter().zip(&rungs) {
        let side = scale * AT.1 as f32;
        let frags = f64::from(CAPACITY) * f64::from(side) * f64::from(side);
        println!(
            "| {scale} | {side:.2} | {:.2} | {:.1}x | {:.3} | {:.3}-{:.3} |",
            frags / 1e6,
            frags / pixels,
            r.ms,
            r.lo,
            r.hi
        );
    }
}

// ---------------------------------------------------------------- ladder 3

fn ladder_element(rig: &mut Rig) {
    let l4 = compile_str("tiny_dots", TINY_DOTS);
    let tc = cost_of("tiny_dots", &l4);
    println!(
        "\ntiny_dots: {} ops/fragment, {} ops/element(vertex), 0.0007 point_rate = 0.5 px",
        tc.ops_per_fragment, tc.ops_per_element
    );
    let mut specs: Vec<Spec> = Vec::new();
    for pad in [0u32, 8, 22, 50, 108, 225, 460] {
        let l1 = compile_str("pad_shell", &pad_shell_src(pad));
        let ops = cost_of("pad_shell", &l1).ops_per_element;
        if specs.iter().any(|p| p.ops == ops) {
            continue;
        }
        specs.push(Spec {
            label: "pad_shell + tiny_dots".into(),
            dial: format!("{pad} curl loop"),
            ops,
            l1,
            fields: vec![],
            l4: l4.clone(),
            edges: vec![],
            warm: 0,
            params: vec![],
        });
    }
    let (rungs, m) = run_ladder(rig, &specs);
    guard("element", SHIPPED_MAX_OPS_PER_ELEMENT, &rungs);
    print_table(
        "Ladder 3 - element",
        "drift_shell with a loop of curl calls in `element`, against a near-free \
         per-element renderer (a fullscreen one would have measured nothing: \
         `Set::step` skips the simulation when every renderer is fullscreen)",
        SHIPPED_MAX_OPS_PER_ELEMENT,
        "ops/element",
        &rungs,
        &m,
    );
}

// ---------------------------------------------------------------- ladder 4

fn ladder_spawn(rig: &mut Rig) {
    let l4 = compile_str("tiny_dots", TINY_DOTS);
    // `spawn_rate` at its declared minimum spawns nothing at all, so the
    // interesting rungs are the default and the maximum.
    for rate in [26_000.0f32, 60_000.0] {
        let mut specs: Vec<Spec> = Vec::new();
        for pad in [0u32, 60, 250, 1000, 4000] {
            let l1 = compile_str("pad_fountain", &pad_fountain_src(pad));
            let ops = cost_of("pad_fountain", &l1).ops_per_spawn;
            if specs.iter().any(|p| p.ops == ops) {
                continue;
            }
            specs.push(Spec {
                label: "pad_fountain + tiny_dots".into(),
                dial: format!("{pad} curl loop @ rate {rate}"),
                ops,
                l1,
                fields: vec![],
                l4: l4.clone(),
                edges: vec![],
                warm: 300,
                params: vec![("spawn_rate".to_string(), rate)],
            });
        }
        let (rungs, m) = run_ladder(rig, &specs);
        guard("spawn", SHIPPED_MAX_OPS_PER_SPAWN, &rungs);
        print_table(
            &format!(
                "Ladder 4 - spawn, spawn_rate {rate} ({:.0} spawns/frame requested)",
                rate / 60.0
            ),
            "warmed 300 steps to a steady population; the quantity is the delta \
             against the trivial-spawn rung at the same rate",
            SHIPPED_MAX_OPS_PER_SPAWN,
            "ops/spawn",
            &rungs,
            &m,
        );
    }
}

fn main() {
    let mut rig = Rig::new();
    let method = {
        // A zero-cost probe of the instrument itself: report what the probe
        // settled on before any figure is printed.
        format!("{:?}", MeasurementMethod::HostWallClock)
    };
    println!("# Ceiling ladder");
    println!(
        "reference workload: {CAPACITY} elements at {}x{}; share used downstream: \
         4.15 ms (16.6 / 4). Constants in crates/karakuri-ir/src/cost.rs are patched \
         to u64::MAX/16 in an uncommitted tree for the duration of this run.",
        AT.0, AT.1
    );
    let _ = method;

    let which = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    if which == "all" || which == "1" {
        ladder_fullscreen(&mut rig);
        ladder_field_march(&mut rig);
    }
    if which == "all" || which == "2" {
        ladder_element_fragment(&mut rig);
    }
    if which == "all" || which == "3" {
        ladder_element(&mut rig);
    }
    if which == "all" || which == "4" {
        ladder_spawn(&mut rig);
    }
    if which == "all" || which == "r" {
        repeatability(&mut rig);
    }
}

// ------------------------------------------------- repeatability (mode "r")

/// Intra-rung measurement spread threshold.
/// rungs is a finding. Also interleaves the one pair of matched-estimate rungs
/// whose order inverted at the top of ladder 1, to see whether that inversion
/// survives repetition.
fn repeatability(rig: &mut Rig) {
    println!("\n## Repeatability — the spread within a rung");
    println!("| configuration | run | ms |");
    println!("|---|---|---|");

    let shell = compile_file("examples/drift_shell.kir");
    let melt = compile_file("examples/melt_blob.kir");
    let beam = compile_file("examples/beam_lattice.kir");
    let melt_edges = [Edge {
        node: "field_lens".into(),
        slot: "shape".into(),
        to: "melt_blob".into(),
    }];
    let beam_edges = [Edge {
        node: "field_lens".into(),
        slot: "shape".into(),
        to: "beam_lattice".into(),
    }];
    let lens40 = compile_str("field_lens", &field_lens_src(40));
    let lens33 = compile_str("field_lens", &field_lens_src(33));
    let lens128 = compile_str("field_lens", &field_lens_src(128));
    let lens107 = compile_str("field_lens", &field_lens_src(107));
    let pad0 = compile_str("pad_points", &pad_points_src(0));
    let tiny = compile_str("tiny_dots", TINY_DOTS);

    for run in 0..5 {
        for (label, l1, fields, l4, edges) in [
            (
                "field_lens@40 + melt_blob (4378 ops)",
                &shell,
                &[&melt][..],
                &lens40,
                &melt_edges[..],
            ),
            (
                "field_lens@33 + beam_lattice (4322 ops)",
                &shell,
                &[&beam][..],
                &lens33,
                &beam_edges[..],
            ),
            (
                "field_lens@128 + melt_blob (13794 ops)",
                &shell,
                &[&melt][..],
                &lens128,
                &melt_edges[..],
            ),
            (
                "field_lens@107 + beam_lattice (13794 ops)",
                &shell,
                &[&beam][..],
                &lens107,
                &beam_edges[..],
            ),
            (
                "drift_shell + pad_points (59 ops/frag, 4 px)",
                &shell,
                &[][..],
                &pad0,
                &[][..],
            ),
            (
                "drift_shell + tiny_dots (0.5 px)",
                &shell,
                &[][..],
                &tiny,
                &[][..],
            ),
        ] {
            let m = rig.measure(label, l1, fields, l4, edges, 0, &[]);
            println!("| {label} | {run} | {:.3} |", m.ms);
            eprintln!("  repeat {run}: {label}: {:.3} ms", m.ms);
        }
    }
}
