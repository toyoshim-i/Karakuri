use karakuri_engine::camera::Orbit;
use karakuri_engine::{Gpu, Set};

use super::fixtures::*;

/// **The camera reaches the frame.** Nothing about it is on the host any more —
/// the state goes into a buffer, a pass derives the matrix, and a bind group
/// carries it to the vertex stage — so a picture that moves when the camera does
/// is the only proof that all three happened.
///
/// Dollying in rather than pitching up, because a pitch rotates about the point
/// the camera looks at and this material is close to it: raising the eye by 1.5
/// moves the material half a texel, which is a fact about the geometry and not
/// about the camera. Distance scales the whole offset and cannot be cancelled by
/// where the material happens to sit.
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

/// **And it is derived every frame**, not once at build.
///
/// An orbit that turns is the case a cached derivation gets wrong, and it gets
/// it wrong silently: the first frame is correct, so a still fixture and a
/// single-frame test both pass. This one lets the same Set run on.
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

/// **The aspect ratio reaches the projection**, and it is the piece most easily
/// lost: it belongs to the canvas rather than to the camera, so it arrives at
/// the derivation from its own buffer, written by its own call, and every test
/// above passes with it stuck at 1.
///
/// Same width, twice the height. The field of view is vertical, so a taller
/// frame at a fixed width is a *narrower* one horizontally — the same world
/// spreads over twice as many texels across, and the material's distance from
/// the centre column doubles.
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

    // **A procedure's map and the built-in's, side by side.** `L3:0` is the
    // `sweep` this Set names and `L3:1` is the orbit after it, which
    // declares the three placement numbers the engine states for it
    // (ADR-0318) — so this asserts two things at once: each camera's params
    // are reported at that camera's address, and the built-in's are its own
    // rather than a procedure's.
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

/// **A camera between the deformations and the renderers does not shift what a
/// renderer reads.** This is a regression: the L4 uniform pass spelled out its
/// own slot arithmetic instead of asking [`Set::slot_of`], so inserting an L3
/// gave every renderer the node before it — and `soft_points` drew a black
/// frame, because its `exposure` resolved against the camera's parameter map
/// and came back missing.
///
/// The reading is a brightness rather than a position, on purpose: a shifted map
/// leaves a declared param with no value, which the uniform path writes as
/// `0.0`. A renderer whose colour *is* its param then goes black — which is
/// exactly what happened, and is the one symptom a picture can show.
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

/// **The built-in orbit is not a second producer of a procedure's camera.** A
/// Set whose files declare an L3 still has the `camera` field on it — a
/// `camera` record and a Set file both set one — and it writes the orbit's own
/// node, which is a different edge: this renderer declares no slot, so it draws
/// from `L3:0`, which is the procedure. Two producers writing *one* edge would
/// resolve by whichever ran last, which is what having a node apiece prevents.
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

/// **An address past a layer's last node reaches nothing**, rather than the
/// first node of the layer after it.
///
/// The parameter maps are laid end to end in node order, so `slot_of(layer) +
/// index` is a position and says nothing about whose it is. A Set with no
/// camera makes that concrete: with nothing between the deformations and the
/// renderers, `L3` and `L4` start at the same slot, and `--param
/// L3:0:exposure=0.0` reached renderer 0 and blacked out the frame — silently,
/// because the caller only reports an address that reached *zero* nodes.
///
/// Two addresses, and the second is the same defect without an L3 in it:
/// `L4:1:` on a Set of one renderer. That one is safe today only because the
/// renderers are last and their range runs to the end of the list, which is a
/// property of the ordering rather than of the check.
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

/// **A vector param is driven by component, and used to panic the render
/// thread for being declared at all.**
///
/// Two defects, one after the other, and this is the test that has watched
/// both. First: every node's uniform path wrote *every* declared name as an
/// `f32`, and the packer panics on a field its layout says is a
/// `vec3<f32>` — so a `.kir` that parses, checks and costs took the render
/// thread down on the first `prepare`, and not in the swap worker, so not
/// caught as `SetError::Panicked`. That was closed by writing the field as
/// a vector, with zeroes, because nothing could state the value.
///
/// Second: the zeroes. `Param::default_scalar` folds a scalar, so a vector
/// never entered a node's value map and a declared `vec3(0.5, 0.5, 0.5)`
/// reached the shader as `vec3(0.0)`. The map holds one `f32` per component
/// now — `centre.x`, `centre.y`, `centre.z`
/// (`docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md`).
///
/// The bare name still holds nothing, and that is the part that did not
/// change: it names three numbers and `Set::param` answers with one.
///
/// Building and preparing is still most of the test: the panic was
/// unconditional.
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

/// **A renderer that declares no slot reads the Set's camera**, which is the
/// first one — and that is what `camera`, `eye` and `ray` have always meant.
///
/// Every renderer in the library is this one, so it is the case that must not
/// have moved: the slot is how a renderer says *which*, and saying nothing has
/// to keep meaning what it meant.
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

/// **The built-in camera is a node, and an edge can name it.**
///
/// It was a field on the `Set` and reachable from nowhere: a Set with no L3 had
/// no L3 node at all, so a renderer could draw from the orbit only by saying
/// nothing. Now it is `orbit` — a name like any other — and the proof that the
/// edge reached the *producer* is that moving the orbit moves the material.
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

/// **The built-in camera is addressable as `L3:0`, and `L4:0` still reaches the
/// first renderer.**
///
/// This is the off-by-one this commit could have introduced. `slot_of` computes
/// a layer's origin by summing the layers before it, so giving the camera layer
/// a node in a Set that had none shifts every renderer's parameter map by one —
/// unless [`Set::params`] grows an entry at the same position, which is a
/// different file's job. Get it wrong and `--param L4:0:gain` writes the
/// camera's map and the renderer keeps its default, silently.
#[test]
fn the_built_in_camera_takes_a_slot_without_moving_the_renderers() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = with_camera(&gpu, None, GAIN_DOT, 64, 64);

    assert_eq!(
        set.node_named(karakuri_engine::set::BUILTIN_CAMERA),
        Some((karakuri_ir::Kind::L3, 0)),
        "a Set with no camera procedure holds the built-in as its L3 node"
    );
    // The renderer is still the first node of L4, and its params are still
    // reported as its own — beside the camera's three, which are reported
    // at `L3:0` and not at the renderer's address (ADR-0318). **That is
    // what makes this test sharper rather than weaker**: the camera's map
    // is no longer empty, so an origin off by one now lands the orbit's
    // `radius` on the renderer instead of landing nothing there.
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

// ----- The built-in camera's three placement numbers ------------------
//
// `docs/adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md`:
// the orbit's `radius`, `speed` and `height` are parameters of the camera
// node, so every route a parameter has reaches them and no route was
// invented for them. These four are the four claims that decision makes,
// and each was watched to fail against the tree that did not carry it.

/// **The built-in camera declares three parameters**, at the values the
/// Set was aimed with and over the ranges the engine states.
///
/// The Set here holds no camera procedure, so `L3:0` is the built-in — and
/// the values are `pinned()`'s rather than `Orbit::default()`'s, which is
/// the second claim in one: `Set::aim_camera` states the three into the
/// node's map and not only into the field beside it.
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

/// **They are published, addressed, in the order the orbit states them**,
/// and over the declared ranges — which is what a fader draws and what a
/// MIDI control is learned against.
///
/// **Addressed and not bare**, which is the part with a picture behind it:
/// seven of this repository's example procedures declare a `radius`, so a
/// bare control would weld the camera to a geometry.
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

/// **A write to `L3:0:radius` reaches the frame**, which is the whole
/// claim: a row that emits a write nothing draws is a row that does
/// nothing.
///
/// The same measurement `a_camera_procedure_produces_the_view` makes, with
/// the built-in as the producer instead of an L3 — so it is the plumbing
/// from the parameter map to the state buffer that is under test and
/// nothing else.
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
