//! Benchmarks slot cost scaling across decreasing render target resolutions.
//! Evaluates sub-pixel rendering floors, `Lines` area tracking, and extrapolation overhead.

use karakuri_engine::probe::{Measurement, MeasurementMethod};
use karakuri_engine::set::{Edge, Layering, Wiring};
use karakuri_engine::{Gpu, Probe, Set, Signals};
use karakuri_ir::typed::Checked;

/// Target resolution used for benchmark measurements.
const AT: (u32, u32) = (1280, 720);

const CAPACITY: u32 = 262_144;
const SEED: u32 = 19_274;

/// Passes over the whole ladder. The first is discarded: a process's very
/// first measurement runs on a cold device and reads low.
const PASSES: u32 = 3;

/// The sizes, 16:9 throughout and halving in area at each step from the
/// reference. 1280x720 is [`AT`] — what every other figure in
/// this repository is quoted at — and 112x63 is the console's preview cell,
/// which is where `tests/deck.rs` measured the floor.
const SIZES: [(u32, u32); 8] = [
    (1280, 720),
    (905, 509),
    (640, 360),
    (452, 254),
    (320, 180),
    (226, 127),
    (160, 90),
    (112, 63),
];

fn compile_str(what: &str, src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{what}: parse: {e:?}"));
    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{what}: check: {e:?}"))
}

fn compile_file(path: &str) -> Checked {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    compile_str(path, &src)
}

/// A per-element L4 that costs as little as a renderer can, at half a pixel so
/// its fragment count is the floored one per element at every size. The
/// control: what is left when the fragments are taken away.
const TINY_DOTS: &str = r#"
proc tiny_dots {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.0007;
  }

  fragment {
    color = vec4(0.01, 0.01, 0.01, 1.0);
  }
}
"#;

/// One material, built once and measured at every size. Built once rather than
/// per rung because a rebuild is a whole-capacity upload and a shader compile,
/// and neither is what this is timing; `Set::rewind` after each run puts the
/// buffers back to what `build` left, which is what the shipped path does too.
struct Material {
    label: &'static str,
    /// What the extrapolation rule would call it, for the table.
    topology: &'static str,
    /// The dial that was turned, and the primitive size it produces at 720p.
    dial: String,
    set: Set,
    capacity: u32,
}

struct Rig {
    gpu: Gpu,
    probe: Probe,
}

impl Rig {
    fn new() -> Rig {
        let gpu = Gpu::headless().expect("no GPU available");
        let probe = Probe::new(&gpu.device, &gpu.queue, gpu.timestamps, AT);
        Rig { gpu, probe }
    }

    fn build(
        &self,
        l1: &Checked,
        l3s: &[&Checked],
        l4: &Checked,
        capacity: u32,
        params: &[(&str, f32)],
    ) -> Set {
        let mut set = Set::build_many(
            &self.gpu.device,
            &self.gpu.queue,
            &[(l1, capacity)],
            &[],
            l3s,
            &[],
            &[l4],
            Layering::Overdraw,
            SEED,
            &[],
            Wiring {
                edges: &[] as &[Edge],
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("{}: Set::build_many: {e}", l1.name));
        for (name, value) in params {
            let n = set.set_param(name, *value);
            assert!(n > 0, "{}: no node declares param `{name}`", l1.name);
        }
        set
    }

    /// One measurement of one material at one size, leaving the Set as it was
    /// found — the same shape as `swap::measure`, so what is timed here is what
    /// the shipped path would pay.
    fn measure(&mut self, m: &mut Material, size: (u32, u32)) -> Measurement {
        self.probe.resize(&self.gpu.device, size);
        m.set.resize(&self.gpu.device, size.0, size.1);
        m.set.prepare(&self.gpu.queue, 1, &Signals::default());
        let out = self
            .probe
            .run(&self.gpu.device, &self.gpu.queue, &mut m.set, 1, m.capacity);
        m.set.rewind(&self.gpu.device, &self.gpu.queue);
        out
    }
}

struct Row {
    size: (u32, u32),
    ms: f32,
    lo: f32,
    hi: f32,
}

/// Median and spread per size, interleaved across materials so that heating
/// lands on every rung rather than on the last one.
fn run_ladder(rig: &mut Rig, materials: &mut [Material]) -> (Vec<Vec<Row>>, MeasurementMethod) {
    run_ladder_over(rig, materials, &SIZES)
}

fn run_ladder_over(
    rig: &mut Rig,
    materials: &mut [Material],
    sizes: &[(u32, u32)],
) -> (Vec<Vec<Row>>, MeasurementMethod) {
    let mut samples: Vec<Vec<Vec<f32>>> = materials
        .iter()
        .map(|_| sizes.iter().map(|_| Vec::new()).collect())
        .collect();
    let mut method = MeasurementMethod::HostWallClock;
    for pass in 0..PASSES {
        for (s, size) in sizes.iter().enumerate() {
            for (i, m) in materials.iter_mut().enumerate() {
                let r = rig.measure(m, *size);
                method = r.method;
                eprintln!(
                    "  pass {pass} | {:>28} | {:>4}x{:<4} | {:.3} ms",
                    m.label, size.0, size.1, r.ms
                );
                if pass > 0 {
                    samples[i][s].push(r.ms);
                }
            }
        }
    }
    let rows = samples
        .into_iter()
        .map(|per_size| {
            per_size
                .into_iter()
                .zip(sizes.iter().copied())
                .map(|(mut xs, size)| {
                    xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
                    Row {
                        size,
                        ms: xs[xs.len() / 2],
                        lo: xs[0],
                        hi: xs[xs.len() - 1],
                    }
                })
                .collect()
        })
        .collect();
    (rows, method)
}

/// `log(ms / ms_reference) / log(area / area_reference)`: 1.0 is "tracks the
/// area", 0.0 is "does not move with it". Printed rather than asserted — the
/// exponent is the whole reading, and a threshold here would be this file
/// deciding the rule instead of reporting it.
fn exponent(reference: &Row, row: &Row) -> f64 {
    let area = |r: &Row| f64::from(r.size.0) * f64::from(r.size.1);
    let ratio_area = area(row) / area(reference);
    let ratio_ms = f64::from(row.ms) / f64::from(reference.ms);
    ratio_ms.ln() / ratio_area.ln()
}

/// Maximum acceptable ratio between high and low pass measurements to detect thermal throttling.
const SPREAD_LIMIT: f32 = 1.25;

fn print_material(m: &Material, rows: &[Row], method: MeasurementMethod) {
    println!("\n## {} — {} — {}", m.label, m.topology, m.dial);
    println!(
        "method: {method:?} | capacity: {} | median of {} interleaved passes",
        m.capacity,
        PASSES - 1
    );
    println!("| target | area vs 720p | ms | spread | ms vs 720p | area exponent |");
    println!("|---|---|---|---|---|---|");
    let reference = &rows[0];
    let ref_area = f64::from(reference.size.0) * f64::from(reference.size.1);
    for r in rows {
        let area = f64::from(r.size.0) * f64::from(r.size.1);
        let exp = if std::ptr::eq(r, reference) {
            String::from("—")
        } else {
            format!("{:.2}", exponent(reference, r))
        };
        println!(
            "| {}x{} | {:.3} | {:.3} | {:.3}-{:.3} | {:.3} | {} |",
            r.size.0,
            r.size.1,
            area / ref_area,
            r.ms,
            r.lo,
            r.hi,
            r.ms / reference.ms,
            exp
        );
    }
    for r in rows {
        if r.hi > r.lo * SPREAD_LIMIT {
            println!(
                "\n**DISCARD THIS RUN.** {}x{} spread {:.3}-{:.3} is wider than {SPREAD_LIMIT:.2}x: \
                 the passes disagree, so what moved between them was the machine and not the \
                 target. Let it cool and run this file alone.",
                r.size.0, r.size.1, r.lo, r.hi
            );
        }
    }
}

/// Resolution rungs evaluated for small-target draw cost extrapolation.
const SMALL_SIZES: [(u32, u32); 5] = [(1280, 720), (905, 509), (640, 360), (452, 254), (320, 180)];

/// A fullscreen L4 with no march and no field: the cheapest thing that can be
/// drawn into an attachment. `Set::step` skips the simulation when every
/// renderer is fullscreen, so what is left is the round trip itself.
const FLAT_FILL: &str = r#"
proc flat_fill {
  kind  L4
  blend additive

  fragment {
    color = vec4(0.01, 0.01, 0.01, 1.0);
  }
}
"#;

fn check_lines(rig: &mut Rig) {
    let shell = compile_file("examples/drift_shell.kir");
    let tunnel = compile_file("examples/warp_tunnel.kir");
    let eye = compile_file("examples/tunnel_eye.kir");
    let soft = compile_file("examples/soft_points.kir");
    let speed = compile_file("examples/speed_lines.kir");
    let march = compile_file("examples/field_march.kir");
    let flat = compile_str("flat_fill", FLAT_FILL);

    let wide: &[(&str, f32)] = &[("width", 0.0333), ("streak", 1.0)];
    let mut materials = vec![
        Material {
            label: "flat_fill",
            topology: "Fullscreen (floor)",
            dial: "submit, wait, clear — no simulation, no march".into(),
            set: rig.build(&shell, &[], &flat, 4096, &[]),
            capacity: 4096,
        },
        Material {
            label: "field_march",
            topology: "Fullscreen (control)",
            dial: "48 march steps".into(),
            set: rig.build(&shell, &[], &march, CAPACITY, &[]),
            capacity: CAPACITY,
        },
        Material {
            label: "warp_tunnel + speed_lines",
            topology: "Lines",
            dial: "width 0.0333 (declared max) = 24 px at 720p, streak 1.0".into(),
            set: rig.build(&tunnel, &[&eye], &speed, CAPACITY, wide),
            capacity: CAPACITY,
        },
        Material {
            label: "warp_tunnel + speed_lines @ 16k",
            topology: "Lines",
            dial: "same, capacity 16384 so the simulation is not the frame".into(),
            set: rig.build(&tunnel, &[&eye], &speed, 16_384, wide),
            capacity: 16_384,
        },
        Material {
            label: "drift_shell + soft_points @ 16k",
            topology: "Points",
            dial: "point_scale 0.0556 (declared max) = 40 px at 720p, capacity 16384".into(),
            set: rig.build(&shell, &[], &soft, 16_384, &[("point_scale", 0.0556)]),
            capacity: 16_384,
        },
    ];

    let (rows, method) = run_ladder_over(rig, &mut materials, &SMALL_SIZES);
    println!(
        "
# Does `Lines` track the target's area?"
    );
    println!(
        "
Every figure `{method:?}`. Read the `ms vs 720p` column against the          `area vs 720p` column: tracking area means the two match. The `floor` row          is what neither of them can go below, and what an area extrapolation          multiplies if it is not subtracted first."
    );
    for (m, r) in materials.iter().zip(&rows) {
        print_material(m, r, method);
    }
    // Affine fit derived from the two smallest rungs to evaluate governor prediction accuracy.
    let floor = rows[0][rows[0].len() - 1].ms;
    println!("\nThe floor — `flat_fill` at its smallest — is {floor:.3} ms.");
    let last = SMALL_SIZES.len() - 1;
    let area = |i: usize| f64::from(SMALL_SIZES[i].0) * f64::from(SMALL_SIZES[i].1);
    println!("\n## The two-rung fit, and what it predicts at 1280x720");
    println!(
        "| material | a (ms) | b*A720 (ms) | predicted 720p | measured 720p | ratio | \
         mid-rung residual |"
    );
    println!("|---|---|---|---|---|---|---|");
    for (m, r) in materials.iter().zip(&rows) {
        let (a1, m1) = (area(last - 1), f64::from(r[last - 1].ms));
        let (a2, m2) = (area(last), f64::from(r[last].ms));
        let b = (m1 - m2) / (a1 - a2);
        let a = m2 - b * a2;
        let pred = a + b * area(0);
        let mid = a + b * area(last - 2);
        println!(
            "| {} | {a:.3} | {:.3} | {pred:.3} | {:.3} | {:.2}x | {:+.1}% |",
            m.label,
            b * area(0),
            r[0].ms,
            pred / f64::from(r[0].ms),
            100.0 * (mid / f64::from(r[last - 2].ms) - 1.0)
        );
    }
    println!(
        "\nThe residual column is the fit evaluated at {}x{} against what was measured \
         there — a third rung the fit did not use, so a large residual says the cost is \
         not affine in the target's area.",
        SMALL_SIZES[last - 2].0,
        SMALL_SIZES[last - 2].1
    );
}

fn main() {
    let mut rig = Rig::new();
    if std::env::args().any(|a| a == "--lines-only") {
        check_lines(&mut rig);
        return;
    }

    let shell = compile_file("examples/drift_shell.kir");
    let tunnel = compile_file("examples/warp_tunnel.kir");
    let eye = compile_file("examples/tunnel_eye.kir");
    let soft = compile_file("examples/soft_points.kir");
    let speed = compile_file("examples/speed_lines.kir");
    let march = compile_file("examples/field_march.kir");
    let tiny = compile_str("tiny_dots", TINY_DOTS);

    let mut materials = vec![
        Material {
            label: "drift_shell + tiny_dots",
            topology: "Points (control)",
            dial: "0.0007 rate = 0.5 px at 720p, floored at every size".into(),
            set: rig.build(&shell, &[], &tiny, 4096, &[]),
            capacity: 4096,
        },
        Material {
            label: "drift_shell + soft_points",
            topology: "Points",
            dial: "point_scale 0.00556 (shipped) = 1.4-4.0 px at 720p".into(),
            set: rig.build(&shell, &[], &soft, CAPACITY, &[]),
            capacity: CAPACITY,
        },
        Material {
            label: "drift_shell + soft_points (fat)",
            topology: "Points",
            dial: "point_scale 0.0222 = 5.6-16 px at 720p, fill-bound".into(),
            set: rig.build(&shell, &[], &soft, CAPACITY, &[("point_scale", 0.0222)]),
            capacity: CAPACITY,
        },
        Material {
            label: "warp_tunnel + speed_lines",
            topology: "Lines",
            dial: "width 0.00139 (shipped) = 1.0 px at 720p, floors below it".into(),
            set: rig.build(&tunnel, &[&eye], &speed, CAPACITY, &[]),
            capacity: CAPACITY,
        },
        Material {
            label: "warp_tunnel + speed_lines (wide)",
            topology: "Lines",
            dial: "width 0.0111 = 8.0 px at 720p, 2.0 px at 320x180".into(),
            set: rig.build(&tunnel, &[&eye], &speed, CAPACITY, &[("width", 0.0111)]),
            capacity: CAPACITY,
        },
        Material {
            label: "field_march",
            topology: "Fullscreen",
            dial: "48 march steps, one fragment per pixel".into(),
            set: rig.build(&shell, &[], &march, CAPACITY, &[]),
            capacity: CAPACITY,
        },
    ];

    let (rows, method) = run_ladder(&mut rig, &mut materials);

    println!("\n# What a small draw costs, by target size");
    println!(
        "\nEvery figure `{:?}`. The exponent column is \
         log(ms ratio) / log(area ratio) against the 1280x720 row: 1.0 is a cost that \
         tracks the target's area, 0.0 is one that does not move with it.",
        method
    );
    for (m, r) in materials.iter().zip(&rows) {
        print_material(m, r, method);
    }

    println!("\n## The floor, stated as a size");
    println!(
        "A primitive is `rate * height` pixels across, so it reaches a pixel at \
         `1 / rate` rows. At the shipped defaults:"
    );
    println!("| material | emitted rate | floor height (rows) |");
    println!("|---|---|---|");
    for (label, rate) in [
        ("soft_points, fastest elements", 0.005_56_f32),
        ("soft_points, slowest elements", 0.005_56 * 0.35),
        ("speed_lines", 0.001_39),
        ("drift_streaks, fastest", 0.001_67),
        ("drift_streaks, slowest", 0.001_67 * 0.35),
        ("any of them, at the declared minimum", 0.000_69),
    ] {
        println!("| {label} | {rate:.5} | {:.0} |", 1.0 / rate);
    }

    check_lines(&mut rig);
}
