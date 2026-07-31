//! Renders the same HDR frame through every tone-mapping operator, at several
//! exposures, so a person can look at the four side by side and pick one.
//!
//! Not a test: there is no correct image to assert against, only material
//! this project actually renders. `examples/spark_fountain.kir` (dense,
//! saturated, additive, with a bright core) is paired with
//! `examples/soft_points.kir` — the pairing `README.md` gives as the
//! non-default one — and `examples/drift_shell.kir` (sparser, the default L1)
//! is included alongside it so the grid also shows the case that motivates
//! AgX the least.
//!
//! `cargo run -p karakuri-engine --example tonemap_compare` writes PNGs under
//! `target/tonemap_compare/`, one per (scene, operator, exposure) triple, plus
//! a second small sweep of `soft_points`' own `exposure` param against a
//! fixed operator — see `material_exposure_sweep` below for why. Run from the
//! repository root: the `.kir` paths are relative to it, same as
//! `karakuri-cli`'s own defaults.

use std::path::{Path, PathBuf};

use karakuri_engine::set::MAX_STEPS;
use karakuri_engine::{Gpu, Present, Set, TonemapOp, VideoSource};
use karakuri_ir::typed::Checked;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
const CAPACITY: u32 = 262_144;
const SEED: u32 = 19_274;

/// Simulation steps to run before capturing, at `dt = 1/60`. `spark_fountain`
/// needs to reach the steady population its lifetime and spawn rate settle at
/// — `README.md` puts that around 74000 of 262144 slots, reached well inside
/// five seconds — and `drift_shell` is run the same distance so both scenes
/// are compared at the same simulation instant rather than at frame zero.
const WARMUP_STEPS: u32 = 300;

/// Parse, check, and cost-estimate one `.kir` file — the same three stages
/// `karakuri-cli`'s `compile::load` runs, minus the diagnostic rendering: a
/// broken example file is a bug in this program, not input to react to.
fn compile(path: &Path) -> Checked {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let proc =
        karakuri_ir::parse(&src).unwrap_or_else(|e| panic!("{}: parse: {e:?}", path.display()));
    let checked = karakuri_ir::check::check(&proc)
        .unwrap_or_else(|e| panic!("{}: check: {e:?}", path.display()));
    karakuri_ir::cost::estimate(&checked)
        .unwrap_or_else(|e| panic!("{}: cost: {e:?}", path.display()));
    checked
}

/// Advances `set` `WARMUP_STEPS` worth of simulation, rendering into
/// `present`'s HDR target as it goes. `MAX_STEPS` caps a single
/// `prepare`/`render` call, so this is a loop of small steps rather than one
/// big one — the same shape a real frame loop uses, just with no wall clock
/// behind it. The HDR target is cleared and redrawn every call, never
/// accumulated across calls, so only the final render's content survives.
fn warm_up(gpu: &Gpu, set: &mut Set, present: &Present) {
    let mut remaining = WARMUP_STEPS;
    while remaining > 0 {
        let steps = remaining.min(u32::from(MAX_STEPS)) as u8;
        set.prepare(&gpu.queue, steps);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        set.render(&mut encoder, present.hdr_view(), steps);
        gpu.queue.submit([encoder.finish()]);
        remaining -= u32::from(steps);
    }
}

/// Selects `op`/`exposure`/`white_point` on `present` — a uniform write, so
/// this never touches a pipeline — then presents whatever is already in its
/// HDR target (set by the last `warm_up` or `Set::render` call) to an
/// `Rgba8UnormSrgb` target and writes it as a PNG. The sRGB encode happens on
/// this write, in hardware, exactly once — the same path `karakuri-cli`'s
/// `--render` takes.
fn capture(gpu: &Gpu, present: &Present, op: TonemapOp, exposure: f32, white_point: f32, path: &Path) {
    present.set_tonemap(&gpu.queue, op, exposure, white_point);

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("tonemap compare target"),
        size: wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());

    let bytes_per_row = WIDTH * 4;
    assert_eq!(bytes_per_row % 256, 0, "width {WIDTH} gives a row that is not 256-aligned");
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("tonemap compare readback"),
        size: u64::from(bytes_per_row * HEIGHT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    present.draw(&mut encoder, &view);
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let pixels = slice.get_mapped_range().to_vec();
    readback.unmap();

    let file = std::fs::File::create(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut png = png::Encoder::new(std::io::BufWriter::new(file), WIDTH, HEIGHT);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .and_then(|mut w| w.write_image_data(&pixels))
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

/// The operator×exposure grid: every operator against every exposure, for one
/// scene, at whatever `exposure` `soft_points.kir` itself declares —
/// unmodified, per the brief not to touch `.kir` files. This is the
/// comparison the four operators exist to support.
fn operator_grid(gpu: &Gpu, present: &Present, out_dir: &Path, scene_name: &str) {
    let operators = [
        ("clamp", TonemapOp::Clamp),
        ("reinhard", TonemapOp::Reinhard),
        ("aces", TonemapOp::Aces),
        ("agx", TonemapOp::AgX),
    ];
    // Reinhard's own control, held fixed across the sweep so only exposure
    // varies between columns: a white-point search is a different question
    // from an exposure search, and conflating them would make the grid
    // unreadable rather than more informative.
    const WHITE_POINT: f32 = 4.0;
    let exposures = [0.25_f32, 0.5, 1.0, 2.0, 4.0];

    for (op_name, op) in operators {
        for exposure in exposures {
            let path = out_dir.join(format!("{scene_name}_{op_name}_e{exposure:.2}.png"));
            capture(gpu, present, op, exposure, WHITE_POINT, &path);
            eprintln!("{}", path.display());
        }
    }
}

/// A second, narrower sweep: `soft_points.kir` carries `param exposure = 0.3`
/// today, a workaround for the tone mapper that did not exist when that
/// default was chosen. This holds the *tone-mapping* exposure fixed at a
/// reasonable value and instead varies the Set's runtime `exposure`
/// parameter — a `--param exposure=...` override, same mechanism the CLI
/// exposes — through the file's current default and a few candidates above
/// it, so the two workarounds (a low material exposure and a missing tone
/// mapper) are not compared against each other by accident.
fn material_exposure_sweep(gpu: &Gpu, l1: &Checked, l4: &Checked, out_dir: &Path) {
    const OP_NAME: &str = "aces";
    const OP: TonemapOp = TonemapOp::Aces;
    const TONEMAP_EXPOSURE: f32 = 1.0;
    let material_exposures = [0.15_f32, 0.30, 0.60, 1.0, 1.4];

    for material_exposure in material_exposures {
        let mut set = Set::build(&gpu.device, &gpu.queue, l1, l4, CAPACITY, SEED)
            .expect("spark_fountain + soft_points must build");
        set.resize(WIDTH, HEIGHT);
        *set
            .params
            .get_mut("exposure")
            .expect("soft_points.kir declares `param exposure`") = material_exposure;

        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba8UnormSrgb, WIDTH, HEIGHT);
        warm_up(gpu, &mut set, &present);

        let path = out_dir.join(format!("spark_fountain_material_exposure_{material_exposure:.2}.png"));
        capture(gpu, &present, OP, TONEMAP_EXPOSURE, 4.0, &path);
        eprintln!("{}", path.display());
    }
    eprintln!(
        "(material exposure sweep uses {OP_NAME} at tonemap exposure {TONEMAP_EXPOSURE:.1}, \
         so only the material's own `exposure` param varies between these frames)"
    );
}

fn main() {
    let gpu = Gpu::headless().expect("no GPU available");
    let out_dir = PathBuf::from("target/tonemap_compare");
    std::fs::create_dir_all(&out_dir).expect("mkdir");

    let l4 = compile(Path::new("examples/soft_points.kir"));

    let scenes: [(&str, &str); 2] = [
        ("drift_shell", "examples/drift_shell.kir"),
        ("spark_fountain", "examples/spark_fountain.kir"),
    ];

    let mut spark_fountain_l1 = None;
    for (scene_name, l1_path) in scenes {
        let l1 = compile(Path::new(l1_path));
        let mut set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, CAPACITY, SEED)
            .unwrap_or_else(|e| panic!("{scene_name} + soft_points: {e}"));
        set.resize(WIDTH, HEIGHT);

        let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba8UnormSrgb, WIDTH, HEIGHT);
        warm_up(&gpu, &mut set, &present);

        operator_grid(&gpu, &present, &out_dir, scene_name);

        if scene_name == "spark_fountain" {
            spark_fountain_l1 = Some(l1);
        }
    }

    material_exposure_sweep(
        &gpu,
        spark_fountain_l1.as_ref().expect("spark_fountain was compiled above"),
        &l4,
        &out_dir,
    );

    eprintln!("wrote comparison frames to {}", out_dir.display());
}
