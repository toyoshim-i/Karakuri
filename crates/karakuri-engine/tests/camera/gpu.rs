use karakuri_engine::camera::Orbit;
use karakuri_engine::{Gpu, Set};

use super::fixtures::*;

/// Verifies that camera translation moves rendered geometry on the frame.
#[test]
fn moving_the_camera_moves_the_material() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;
    let offset = |radius: f32| {
        let mut set = build(&gpu, W, H, Orbit { radius, ..pinned() });
        centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0
    };

    let far = offset(5.0);
    let near = offset(3.0);
    assert!(
        far.abs() > 4.0,
        "the material is on the centre column; nothing to measure"
    );
    // Two thirds the distance, so five thirds the offset — asserted as a
    // direction and a lower bound rather than a ratio, since what is under test
    // is that the camera arrives at all.
    assert!(
        near.abs() > far.abs() * 1.3,
        "closing from 5 to 3 moved the material from {far} texels off centre to \
     {near} — the camera did not reach the draw"
    );
}

/// Verifies that camera transform matrices are recomputed each frame rather than cached.
#[test]
fn a_turning_camera_keeps_turning() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;
    // Thirty frames at sixty steps a second is half a second; at a quarter
    // revolution a second that is an eighth of a turn, which swings the material
    // a good way across the frame without carrying it off the edge.
    let mut set = build(
        &gpu,
        W,
        H,
        Orbit {
            speed: 0.25,
            ..pinned()
        },
    );

    let first = centroid(&gpu, &mut set, W, H).0;
    let mut last = first;
    for _ in 0..30 {
        last = centroid(&gpu, &mut set, W, H).0;
    }
    assert!(
        (last - first).abs() > 6.0,
        "thirty frames of a turning camera left the material at column {last}, \
     where it started at {first} — the derivation ran once and was reused"
    );
}

/// Verifies that canvas aspect ratio propagates into the camera projection matrix.
#[test]
fn the_canvas_shape_reaches_the_projection() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 128;

    let wide = centroid(&gpu, &mut build(&gpu, W, 64, pinned()), W, 64).0 - W as f32 / 2.0;
    let square = centroid(&gpu, &mut build(&gpu, W, 128, pinned()), W, 128).0 - W as f32 / 2.0;

    assert!(
        wide.abs() > 4.0,
        "the material is on the centre column; nothing to measure"
    );
    assert!(
        (square - 2.0 * wide).abs() < 0.25 * wide.abs(),
        "at aspect 2 the material sits {wide} texels from the centre and at aspect 1 \
     it sits {square}, where twice {wide} was due — the canvas did not reach the \
     projection"
    );
}

/// **A camera procedure produces the view**, and its params reach it. The whole
/// path is on the GPU — a uniform write, a compute pass writing six numbers, a
/// second deriving a matrix, and a bind group — so moving the eye and watching
/// the material move is the only end-to-end proof there is.
#[test]
fn a_camera_procedure_produces_the_view() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;

    let offset = |dist: f32| {
        let mut set = with_camera(&gpu, Some(&sweep(dist)), GAIN_DOT, W, H);
        centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0
    };
    let far = offset(5.0);
    let near = offset(3.0);

    assert!(
        far.abs() > 4.0,
        "the material is on the centre column; nothing to measure"
    );
    assert!(
        near.abs() > far.abs() * 1.3,
        "closing the camera's own `dist` from 5 to 3 moved the material from {far} texels off \
     centre to {near} — the procedure did not reach the frame"
    );
}

/// **And it can be addressed after the build**, like any other node's params.
#[test]
fn a_cameras_parameters_are_addressed_as_a_nodes() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;
    let mut set = with_camera(&gpu, Some(&sweep(5.0)), GAIN_DOT, W, H);

    // Verify procedure parameter map and built-in placement parameter map isolation.
    let mut declared: Vec<(u32, &str, f32)> = set
        .params()
        .filter(|(layer, ..)| *layer == karakuri_ir::Kind::L3)
        .map(|(_, index, name, value)| (index, name, value))
        .collect();
    declared.sort_by_key(|(index, name, _)| (*index, *name));
    assert_eq!(
        declared,
        vec![
            (0, "dist", 5.0),
            (1, "height", 0.0),
            (1, "radius", 5.0),
            (1, "speed", 0.0),
        ],
        "the cameras' params are not reported as each camera's own"
    );

    let far = centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0;
    assert!(
        set.set_param_at(karakuri_ir::Kind::L3, 0, "dist", 3.0),
        "the camera declares `dist`"
    );
    let near = centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0;
    assert!(
        near.abs() > far.abs() * 1.3,
        "writing the camera's `dist` left the material at {near}, against {far}"
    );
}

/// Verifies that inserting an L3 camera does not shift uniform binding slots for downstream L4 renderers.
#[test]
fn a_camera_does_not_shift_the_parameters_a_renderer_reads() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 64;
    const H: u32 = 64;

    let peak = |l3: Option<&str>| {
        let mut set = with_camera(&gpu, l3, GAIN_DOT, W, H);
        frame(&gpu, &mut set, W, H)
            .chunks_exact(4)
            .map(|t| t[0])
            .fold(0.0f32, f32::max)
    };
    // The same camera either way, so the only difference between the two Sets
    // is whether a node sits between the geometry and the renderer.
    let built_in = peak(None);
    let procedure = peak(Some(&sweep(5.0)));

    assert!(
        built_in > 0.5,
        "the renderer's own default never reached the frame: {built_in}"
    );
    assert!(
        (procedure - built_in).abs() < 0.01,
        "with a camera procedure the renderer drew at {procedure}, and without one at \
     {built_in} — a node was inserted and the renderer read the map beside its own"
    );
}

/// Verifies that built-in orbit assignments do not overwrite an active L3 camera procedure.
#[test]
fn an_orbit_assigned_beside_a_camera_procedure_reaches_nothing() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;

    let mut set = with_camera(&gpu, Some(&sweep(5.0)), GAIN_DOT, W, H);
    let before = centroid(&gpu, &mut set, W, H);
    // A camera nowhere near the procedure's, and pointed from above rather than
    // level, so anything of it that leaked would move the material a long way.
    set.aim_camera(Orbit {
        radius: 20.0,
        height: 18.0,
        speed: 0.0,
        ..Default::default()
    });
    let after = centroid(&gpu, &mut set, W, H);

    assert!(
        (before.0 - after.0).abs() < 0.5 && (before.1 - after.1).abs() < 0.5,
        "assigning an orbit moved the material from {before:?} to {after:?} — the built-in \
     reached a Set whose camera is a procedure"
    );
}

/// Verifies that addressing past a layer's terminal node returns an error without spilling into adjacent layers.
#[test]
fn an_address_past_a_layers_last_node_reaches_nothing() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = with_camera(&gpu, None, GAIN_DOT, 64, 64);

    assert_eq!(
        set.param("gain"),
        Some(1.0),
        "the renderer's declared default"
    );
    assert!(
        !set.set_param_at(karakuri_ir::Kind::L3, 0, "gain", 0.0),
        "a Set with no camera has no L3 node to address"
    );
    assert_eq!(
        set.param("gain"),
        Some(1.0),
        "the L3 address reached the renderer"
    );
    assert!(
        !set.set_param_at(karakuri_ir::Kind::L4, 1, "gain", 0.0),
        "this Set draws with one renderer, so index 1 addresses nothing"
    );
    assert_eq!(
        set.param("gain"),
        Some(1.0),
        "an out-of-range renderer address wrote anyway"
    );
    // And the address that does exist still works, so the bound is a bound and
    // not a refusal.
    assert!(set.set_param_at(karakuri_ir::Kind::L4, 0, "gain", 0.25));
    assert_eq!(set.param("gain"), Some(0.25));
}

/// Verifies that vector parameters are driven by individual component keys without crashing uniform packing.
#[test]
fn a_vector_param_is_driven_by_component_rather_than_packed_as_a_scalar() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mixed = r#"
proc mixed {
  kind  L4
  blend additive

  param centre : vec3  [0.0, 1.0] = vec3(0.5, 0.5, 0.5)
  param gain   : float [0.0, 4.0] = 1.0

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(gain, gain, gain, 1.0);
  }
}
"#;
    let mut set = with_camera(&gpu, Some(&sweep(5.0)), mixed, 64, 64);
    let peak = frame(&gpu, &mut set, 64, 64)
        .chunks_exact(4)
        .map(|t| t[0])
        .fold(0.0f32, f32::max);

    // And the scalar beside it still arrives, so the filter is a filter rather
    // than a node that gave up on its params.
    assert!(
        peak > 0.5,
        "the scalar param never reached the frame: {peak}"
    );
    assert_eq!(set.param("gain"), Some(1.0));
    assert_eq!(
        set.param("centre"),
        None,
        "the bare name of a vector param names three numbers and holds none"
    );
    for key in ["centre.x", "centre.y", "centre.z"] {
        assert_eq!(
            set.param(key),
            Some(0.5),
            "{key} is what the uniform is packed from, and the declaration states it"
        );
    }
}

/// Asserts that multiple renderers draw from their specifically wired camera nodes
/// and correctly respond to edge swaps.
#[test]
fn two_renderers_draw_from_the_cameras_their_edges_name() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;
    let cameras = [from_x("east", 5.0), from_x("west", -5.0)];
    let red = through("red", [1.0, 0.0, 0.0]);
    let green = through("green", [0.0, 1.0, 0.0]);

    let measure = |edges: &[(&str, &str, &str)]| {
        let mut set = wired(&gpu, &cameras, &[&red, &green], edges, W, H).expect("two cameras");
        let px = frame(&gpu, &mut set, W, H);
        (
            column(&px, W, 0) - W as f32 / 2.0,
            column(&px, W, 1) - W as f32 / 2.0,
        )
    };

    let (r, g) = measure(&[("red", "view", "east"), ("green", "view", "west")]);
    assert!(
        r.abs() > 4.0 && g.abs() > 4.0,
        "the material is on the centre column in one of the two; nothing to measure: {r}, {g}"
    );
    assert!(
        r.signum() != g.signum(),
        "two mirror-image cameras left both renderers on the same side of the frame — \
     {r} and {g} — so both drew from one camera"
    );

    // The same two renderers and the same two cameras, wired the other way
    // round.
    let (r2, g2) = measure(&[("red", "view", "west"), ("green", "view", "east")]);
    assert!(
        (r2 - g).abs() < 1.0 && (g2 - r).abs() < 1.0,
        "swapping the edges left the picture at {r2}, {g2} where {g}, {r} was due — \
     a renderer is reading the camera at its own index rather than the one it names"
    );
}

/// Verifies that renderers declaring no explicit camera slot default to binding the primary camera (index 0).
#[test]
fn a_renderer_with_no_slot_reads_the_sets_camera() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;
    let cameras = [from_x("east", 5.0), from_x("west", -5.0)];
    let bound = through("bound", [1.0, 0.0, 0.0]);

    // The unbound renderer draws blue and the bound one draws red, in one
    // frame, so the two readings come from one build and one camera pass.
    let mut set = wired(
        &gpu,
        &cameras,
        &[&bound, PLAIN],
        &[("bound", "view", "east")],
        W,
        H,
    )
    .expect("a slot and a renderer that declares none");
    let px = frame(&gpu, &mut set, W, H);
    let named = column(&px, W, 0) - W as f32 / 2.0;
    let silent = column(&px, W, 2) - W as f32 / 2.0;

    assert!(
        named.abs() > 4.0,
        "the material is on the centre column; nothing to measure"
    );
    assert!(
        (named - silent).abs() < 1.0,
        "the renderer that named `east` drew at {named} and the one that named nothing at \
     {silent} — a renderer with no slot has to read camera 0, which is `east`"
    );
}

/// Verifies that the built-in orbit camera acts as a node addressable by wiring edges.
#[test]
fn the_built_in_camera_is_a_node_an_edge_can_name() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;
    let named = through("named", [1.0, 0.0, 0.0]);

    let offset = |radius: f32| {
        let mut set = wired(
            &gpu,
            &[],
            &[&named],
            &[("named", "view", karakuri_engine::set::BUILTIN_CAMERA)],
            W,
            H,
        )
        .expect("a renderer bound to the built-in camera");
        set.aim_camera(Orbit { radius, ..pinned() });
        column(&frame(&gpu, &mut set, W, H), W, 0) - W as f32 / 2.0
    };

    let far = offset(5.0);
    let near = offset(3.0);
    assert!(
        far.abs() > 4.0,
        "the material is on the centre column; nothing to measure"
    );
    assert!(
        near.abs() > far.abs() * 1.3,
        "closing the orbit from 5 to 3 moved the material from {far} texels off centre to \
     {near} — the edge did not reach the built-in producer"
    );
}

/// Verifies that the built-in camera occupies L3:0 while preserving L4:0 alignment for the primary renderer.
#[test]
fn the_built_in_camera_takes_a_slot_without_moving_the_renderers() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = with_camera(&gpu, None, GAIN_DOT, 64, 64);

    assert_eq!(
        set.node_named(karakuri_engine::set::BUILTIN_CAMERA),
        Some((karakuri_ir::Kind::L3, 0)),
        "a Set with no camera procedure holds the built-in as its L3 node"
    );
    // Verify L4:0 renderer parameter reporting remains distinct from L3:0 camera parameters (ADR-0318).
    let mut declared: Vec<(karakuri_ir::Kind, u32, &str)> = set
        .params()
        .map(|(layer, index, name, _)| (layer, index, name))
        .collect();
    declared.sort_by_key(|(layer, index, name)| (format!("{layer:?}"), *index, *name));
    assert_eq!(
        declared,
        vec![
            (karakuri_ir::Kind::L3, 0, "height"),
            (karakuri_ir::Kind::L3, 0, "radius"),
            (karakuri_ir::Kind::L3, 0, "speed"),
            (karakuri_ir::Kind::L4, 0, "gain"),
        ],
        "the renderer's params are reported at the renderer's address"
    );

    // And a write at that address reaches it: the value moves and the picture
    // moves with it.
    let peak = |set: &mut Set| {
        frame(&gpu, set, 64, 64)
            .chunks_exact(4)
            .map(|t| t[0])
            .fold(0.0f32, f32::max)
    };
    assert!(peak(&mut set) > 0.5, "the renderer's default never drew");
    assert!(
        set.set_param_at(karakuri_ir::Kind::L4, 0, "gain", 0.0),
        "`L4:0` addresses the first renderer"
    );
    assert!(
        peak(&mut set) < 0.01,
        "writing `L4:0:gain` did not reach the renderer — every L4 address is off by one"
    );
    // The camera's own address reaches the camera, rather than reaching the
    // renderer's map. It declares three parameters of its own since
    // ADR-0318 and `gain` is not one of them, which is what makes this an
    // off-by-one test rather than an empty-map one.
    assert!(
        !set.set_param_at(karakuri_ir::Kind::L3, 0, "gain", 1.0),
        "`L3:0` is the built-in camera, which has no `gain` — an address that writes one \
         has walked into the renderer's map"
    );
}

// Built-in camera placement parameters (radius, speed, height) as parameter rows (ADR-0318).

/// Verifies that the built-in camera exposes radius, speed, and height parameter rows.
#[test]
fn the_built_in_camera_declares_its_three_placement_numbers() {
    let gpu = Gpu::headless().expect("no GPU available");
    let set = with_camera(&gpu, None, GAIN_DOT, 32, 32);

    let mut declared: Vec<(u32, &str, f32)> = set
        .params()
        .filter(|(layer, ..)| *layer == karakuri_ir::Kind::L3)
        .map(|(_, index, key, value)| (index, key, value))
        .collect();
    declared.sort_by_key(|(_, key, _)| *key);
    assert_eq!(
        declared,
        vec![(0, "height", 0.0), (0, "radius", 5.0), (0, "speed", 0.0),],
        "the built-in camera's parameter map is not the orbit it was aimed with"
    );
}

/// Verifies that built-in camera parameters are published in order with explicit layer addresses and ranges.
#[test]
fn the_cameras_three_publish_addressed_and_in_order() {
    let gpu = Gpu::headless().expect("no GPU available");
    let set = with_camera(&gpu, None, GAIN_DOT, 32, 32);

    let mine: Vec<(String, [f32; 2])> = set
        .published()
        .into_iter()
        .filter(|p| Orbit::PLACEMENT.iter().any(|(key, _)| *key == p.key))
        .inspect(|p| {
            assert_eq!(
                p.at,
                Some((karakuri_ir::Kind::L3, 0)),
                "`{}` was published bare, and a bare name is a control over every node \
                 that declares it",
                p.key
            );
        })
        .map(|p| (p.key, p.range))
        .collect();
    assert_eq!(
        mine,
        vec![
            ("radius".to_owned(), [1.0, 40.0]),
            ("speed".to_owned(), [0.0, 2.0]),
            ("height".to_owned(), [-40.0, 40.0]),
        ],
        "the camera's rows are not the three the engine declares, in order"
    );
}

/// Verifies that parameter writes to L3:0:radius update the camera transform and move rendered output.
#[test]
fn a_write_to_the_cameras_radius_reaches_the_frame() {
    let gpu = Gpu::headless().expect("no GPU available");
    const W: u32 = 96;
    const H: u32 = 96;

    let offset = |radius: f32| {
        let mut set = with_camera(&gpu, None, GAIN_DOT, W, H);
        assert!(
            set.write_param(&karakuri_engine::ParamWrite::at(
                karakuri_ir::Kind::L3,
                0,
                "radius",
                radius,
            ))
            .expect("one node, so no authority to cross")
                == 1,
            "the write did not land on the camera node"
        );
        centroid(&gpu, &mut set, W, H).0 - W as f32 / 2.0
    };
    let far = offset(5.0);
    let near = offset(3.0);

    assert!(
        far.abs() > 4.0,
        "the material is on the centre column; nothing to measure"
    );
    assert!(
        near.abs() > far.abs() * 1.3,
        "closing the camera's radius from 5 to 3 moved the material from {far} texels off \
         centre to {near} — the parameter did not reach the frame"
    );
}

/// **A bare name does not reach it**, which is the other half of the row
/// above and the one with a defect behind it: `drift_shell` declares a
/// `radius` of its own, so a `--param radius=…` that also swung the camera
/// would be a control doing something it does not draw.
#[test]
fn a_bare_name_does_not_reach_the_cameras_radius() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = with_camera(&gpu, None, GAIN_DOT, 32, 32);

    let landed = set
        .write_param(&karakuri_engine::ParamWrite::everywhere("radius", 2.0))
        .expect("nothing to cross");
    assert_eq!(
        landed, 0,
        "a bare `radius` landed somewhere, and the only node declaring one here is the camera"
    );
    assert_eq!(set.orbit().radius, 5.0, "the camera moved on a bare name");
}

/// Asserts that a Set rebuild retains operator-ridden camera parameters (ADR-0132, ADR-0282)
/// while restating unchanged declarative settings.
#[test]
fn a_rebuild_keeps_a_ridden_camera_and_restates_the_rest() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut outgoing = with_camera(&gpu, None, GAIN_DOT, 32, 32);
    outgoing
        .write_param(&karakuri_engine::ParamWrite::at(
            karakuri_ir::Kind::L3,
            0,
            "radius",
            12.0,
        ))
        .expect("nothing to cross");

    // What the request would state: the aim the slot is pointed at, which
    // is `pinned()` with a wider lens than the ride ever touches.
    let restated = Orbit {
        fov_y: 1.0,
        ..pinned()
    };
    let mut incoming = with_camera(&gpu, None, GAIN_DOT, 32, 32);
    incoming.aim_camera(restated);
    assert_eq!(
        incoming.orbit().radius,
        5.0,
        "the restatement is what a fresh build holds before anything is carried"
    );

    assert_eq!(
        incoming.carry_moved_from(&outgoing),
        1,
        "one value was moved, so one is carried"
    );
    assert_eq!(
        incoming.orbit().radius,
        12.0,
        "the ridden radius did not survive the rebuild"
    );
    assert_eq!(
        incoming.orbit().fov_y,
        1.0,
        "the lens came from the outgoing Set rather than from what was restated"
    );
    assert_eq!(
        incoming.orbit().speed,
        pinned().speed,
        "a number nobody moved came from somewhere other than the restatement"
    );
}
