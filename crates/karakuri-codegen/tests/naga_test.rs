//! Feeds generated WGSL through a real front end.
//!
//! Everything in `src/lib.rs`'s unit tests asserts on the emitted *text*,
//! which only proves the generator produced what it meant to produce — not
//! that a GPU driver would accept it. `naga` is already in the dependency
//! tree via `wgpu` (`karakuri-engine`); pulling it in directly here lets
//! these tests parse and validate the output instead of trusting that it
//! merely looks right. A generator whose output is never fed to a compiler
//! emits plausible nonsense, and this is the test that would have caught
//! this crate doing that.
//!
//! The two fixtures below are hand-built `Checked` trees shaped like the
//! ir-spec's own `drift_shell` (L1) and `soft_points` (L4) examples, close
//! enough to exercise hashing, noise, curl, an `if`/`kill()` branch, `mat4`
//! multiplication, and `hsv_to_rgb` together — not just the narrow feature
//! one unit test isolates.

use karakuri_ir::builtin::Builtin;
use karakuri_ir::typed::{Checked, TBlock, TExpr, TExprKind, TStmt, Target};
use karakuri_ir::{
    Ambient, Attr, BinOp, Blend, BlockKind, Kind, Lit, Output, Param, Span, Topology, Ty,
};

fn span() -> Span {
    Span::EMPTY
}

fn lit_f(f: f32) -> TExpr {
    TExpr::new(Ty::Float, span(), TExprKind::Lit(Lit::Float(f)))
}

fn lit_u(u: u32) -> TExpr {
    TExpr::new(Ty::Uint, span(), TExprKind::Lit(Lit::Uint(u)))
}

fn attr(a: Attr) -> TExpr {
    TExpr::new(a.ty(), span(), TExprKind::Attr(a))
}

fn ambient(a: Ambient, ty: Ty) -> TExpr {
    TExpr::new(ty, span(), TExprKind::Ambient(a))
}

fn local(name: &str, ty: Ty) -> TExpr {
    TExpr::new(ty, span(), TExprKind::Local(name.to_string()))
}

fn param(name: &str, ty: Ty) -> TExpr {
    TExpr::new(ty, span(), TExprKind::Param(name.to_string()))
}

fn bin(op: BinOp, lhs: TExpr, rhs: TExpr, ty: Ty) -> TExpr {
    TExpr::new(
        ty,
        span(),
        TExprKind::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
    )
}

fn call(func: Builtin, args: Vec<TExpr>, ty: Ty) -> TExpr {
    TExpr::new(ty, span(), TExprKind::Builtin { func, args })
}

fn construct(ty: Ty, args: Vec<TExpr>) -> TExpr {
    TExpr::new(ty, span(), TExprKind::Construct { args })
}

fn let_(name: &str, value: TExpr) -> TStmt {
    TStmt::Let {
        name: name.to_string(),
        value,
        span: span(),
    }
}

fn assign_attr(a: Attr, value: TExpr) -> TStmt {
    TStmt::Assign {
        target: Target::Attr(a),
        value,
        span: span(),
    }
}

fn assign_output(o: Output, value: TExpr) -> TStmt {
    TStmt::Assign {
        target: Target::Output(o),
        value,
        span: span(),
    }
}

fn param_decl(name: &str, ty: Ty, min: f32, max: f32) -> Param {
    Param {
        name: name.to_string(),
        ty,
        min,
        max,
        default: dummy_default(),
        span: span(),
    }
}

fn dummy_default() -> karakuri_ir::Expr {
    karakuri_ir::Expr::Lit {
        value: Lit::Float(0.0),
        span: span(),
    }
}

/// Shaped like the ir-spec's `drift_shell` L1 example.
fn drift_shell() -> Checked {
    let spawn = TBlock {
        kind: BlockKind::Spawn,
        span: span(),
        stmts: vec![
            // Named `u` and `v`, verbatim, exactly as the ir-spec's own
            // `drift_shell` writes them (`let u = hash1(seed); let v =
            // hash1(seed + 1000u);`). This is the regression: `u` is also
            // the generated uniform binding's name, and `v` is what the
            // element block below separately calls its own `let`. Neither
            // may capture anything this crate emits.
            let_(
                "u",
                call(
                    Builtin::Hash1,
                    vec![ambient(Ambient::Seed, Ty::Uint)],
                    Ty::Float,
                ),
            ),
            let_(
                "v",
                call(
                    Builtin::Hash1,
                    vec![bin(
                        BinOp::Add,
                        ambient(Ambient::Seed, Ty::Uint),
                        lit_u(1000),
                        Ty::Uint,
                    )],
                    Ty::Float,
                ),
            ),
            assign_attr(
                Attr::Position,
                bin(
                    BinOp::Mul,
                    call(
                        Builtin::SpherePoint,
                        vec![local("u", Ty::Float), local("v", Ty::Float)],
                        Ty::Vec3,
                    ),
                    param("radius", Ty::Float),
                    Ty::Vec3,
                ),
            ),
            assign_attr(Attr::Velocity, construct(Ty::Vec3, vec![lit_f(0.0)])),
            assign_attr(Attr::Age, lit_f(0.0)),
        ],
    };

    let flow = call(
        Builtin::Curl,
        vec![bin(
            BinOp::Add,
            bin(BinOp::Mul, attr(Attr::Position), lit_f(0.3), Ty::Vec3),
            construct(
                Ty::Vec3,
                vec![
                    lit_f(0.0),
                    bin(
                        BinOp::Mul,
                        ambient(Ambient::T, Ty::Float),
                        lit_f(0.1),
                        Ty::Float,
                    ),
                    lit_f(0.0),
                ],
            ),
            Ty::Vec3,
        )],
        Ty::Vec3,
    );
    let flow = bin(BinOp::Mul, flow, param("turbulence", Ty::Float), Ty::Vec3);
    let new_v = bin(
        BinOp::Add,
        bin(BinOp::Mul, attr(Attr::Velocity), lit_f(0.96), Ty::Vec3),
        bin(
            BinOp::Mul,
            local("flow", Ty::Vec3),
            ambient(Ambient::Dt, Ty::Float),
            Ty::Vec3,
        ),
        Ty::Vec3,
    );
    let element = TBlock {
        kind: BlockKind::Element,
        span: span(),
        stmts: vec![
            let_("flow", flow),
            let_("v", new_v),
            assign_attr(Attr::Velocity, local("v", Ty::Vec3)),
            assign_attr(
                Attr::Position,
                bin(
                    BinOp::Add,
                    attr(Attr::Position),
                    bin(
                        BinOp::Mul,
                        local("v", Ty::Vec3),
                        ambient(Ambient::Dt, Ty::Float),
                        Ty::Vec3,
                    ),
                    Ty::Vec3,
                ),
            ),
            assign_attr(
                Attr::Age,
                bin(
                    BinOp::Add,
                    attr(Attr::Age),
                    ambient(Ambient::Dt, Ty::Float),
                    Ty::Float,
                ),
            ),
            TStmt::If {
                cond: bin(
                    BinOp::Gt,
                    attr(Attr::Age),
                    param("lifetime", Ty::Float),
                    Ty::Bool,
                ),
                then: vec![TStmt::Kill { span: span() }],
                els: vec![],
                span: span(),
            },
        ],
    };

    Checked {
        name: "drift_shell".to_string(),
        kind: Kind::L1,
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: None,
        params: vec![
            param_decl("spawn_rate", Ty::Float, 0.0, 40000.0),
            param_decl("radius", Ty::Float, 0.1, 8.0),
            param_decl("turbulence", Ty::Float, 0.0, 3.0),
            param_decl("lifetime", Ty::Float, 0.5, 20.0),
        ],
        emit: vec![Attr::Position, Attr::Velocity, Attr::Age],
        consumes: vec![],
        blocks: vec![spawn, element],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// Shaped like the ir-spec's `soft_points` L4 example.
fn soft_points() -> Checked {
    let clip = bin(
        BinOp::Mul,
        ambient(Ambient::Camera, Ty::Mat4),
        construct(Ty::Vec4, vec![attr(Attr::Position), lit_f(1.0)]),
        Ty::Vec4,
    );
    let speed_term = bin(
        BinOp::Mul,
        lit_f(0.2),
        call(Builtin::Length, vec![attr(Attr::Velocity)], Ty::Float),
        Ty::Float,
    );
    let clamped = call(
        Builtin::Clamp,
        vec![speed_term, lit_f(0.0), lit_f(1.0)],
        Ty::Float,
    );
    let point_size = bin(
        BinOp::Mul,
        param("point_scale", Ty::Float),
        bin(
            BinOp::Add,
            lit_f(0.3),
            bin(BinOp::Mul, lit_f(0.7), clamped, Ty::Float),
            Ty::Float,
        ),
        Ty::Float,
    );
    let vertex = TBlock {
        kind: BlockKind::Vertex,
        span: span(),
        stmts: vec![
            assign_output(Output::Clip, clip),
            assign_output(Output::PointSize, point_size),
        ],
    };

    let d = call(
        Builtin::Length,
        vec![bin(
            BinOp::Sub,
            bin(
                BinOp::Mul,
                ambient(Ambient::PointCoord, Ty::Vec2),
                lit_f(2.0),
                Ty::Vec2,
            ),
            construct(Ty::Vec2, vec![lit_f(1.0)]),
            Ty::Vec2,
        )],
        Ty::Float,
    );
    let a = call(
        Builtin::Pow,
        vec![
            call(
                Builtin::Max,
                vec![
                    lit_f(0.0),
                    bin(BinOp::Sub, lit_f(1.0), local("d", Ty::Float), Ty::Float),
                ],
                Ty::Float,
            ),
            param("falloff", Ty::Float),
        ],
        Ty::Float,
    );
    let hue_jitter = bin(
        BinOp::Add,
        param("hue", Ty::Float),
        bin(
            BinOp::Mul,
            call(
                Builtin::Hash1,
                vec![ambient(Ambient::Seed, Ty::Uint)],
                Ty::Float,
            ),
            lit_f(0.05),
            Ty::Float,
        ),
        Ty::Float,
    );
    let c = call(
        Builtin::HsvToRgb,
        vec![construct(
            Ty::Vec3,
            vec![hue_jitter, lit_f(0.7), lit_f(1.0)],
        )],
        Ty::Vec3,
    );
    let color = construct(
        Ty::Vec4,
        vec![
            bin(
                BinOp::Mul,
                local("c", Ty::Vec3),
                param("exposure", Ty::Float),
                Ty::Vec3,
            ),
            local("a", Ty::Float),
        ],
    );
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: vec![
            let_("d", d),
            let_("a", a),
            let_("c", c),
            assign_output(Output::Color, color),
        ],
    };

    Checked {
        name: "soft_points".to_string(),
        kind: Kind::L4,
        // Not `None`: an L4's topology is inferred by the check pass, and
        // `generate_l4` reads it to choose the quad expansion. These are
        // hand-built stand-ins for checked trees, so they carry what `check`
        // would have put here.
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: Some(Blend::Additive),
        params: vec![
            param_decl("point_scale", Ty::Float, 0.5, 40.0),
            param_decl("hue", Ty::Float, 0.0, 1.0),
            param_decl("exposure", Ty::Float, 0.0, 8.0),
            param_decl("falloff", Ty::Float, 0.5, 8.0),
        ],
        emit: vec![],
        consumes: vec![Attr::Position, Attr::Velocity, Attr::Age],
        blocks: vec![vertex, fragment],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// An L1 procedure whose `let`s are named after every bare identifier this
/// crate's L1 lowering emits: the uniform binding (`u`), the engine-state
/// bindings (`counts`, `dest`, `step_args`) and their fields (`range`,
/// `survivors`, `spawn_count`, `seed_base`), entry-point locals (`seed`,
/// `i`, `out`, `slot`, `gid`, `birth_frac`), ambient-backed uniform fields
/// (`t`, `dt`, `capacity`), and helper function names (`hash1`,
/// `sphere_point`, `curl`, `mod_f32`). None of these are contrived: `u` is
/// the ir-spec's own `drift_shell` (see `drift_shell` above); the rest are
/// exactly as plausible for an LLM to reach for, since none of them is a
/// reserved word in the `.kir` grammar. The block still exercises `hash1`,
/// `sphere_point`, `curl`, and `%` on a float for real afterwards — the
/// point is that declaring a local of the same name earlier must not have
/// broken any of them.
fn shadowing_locals_l1() -> Checked {
    let adversarial_lets = [
        "u",
        "hash1",
        "seed",
        "prev_position",
        "next_age",
        "i",
        "out",
        "slot",
        "gid",
        "sphere_point",
        "curl",
        "birth_frac",
        "counts",
        "dest",
        "step_args",
    ];
    let mut spawn_stmts: Vec<TStmt> = adversarial_lets
        .iter()
        .enumerate()
        .map(|(n, name)| let_(name, lit_f(n as f32)))
        .collect();
    spawn_stmts.push(assign_attr(
        Attr::Position,
        bin(
            BinOp::Mul,
            call(
                Builtin::SpherePoint,
                vec![
                    call(
                        Builtin::Hash1,
                        vec![ambient(Ambient::Seed, Ty::Uint)],
                        Ty::Float,
                    ),
                    call(
                        Builtin::Hash1,
                        vec![bin(
                            BinOp::Add,
                            ambient(Ambient::Seed, Ty::Uint),
                            lit_u(7),
                            Ty::Uint,
                        )],
                        Ty::Float,
                    ),
                ],
                Ty::Vec3,
            ),
            param("radius", Ty::Float),
            Ty::Vec3,
        ),
    ));
    spawn_stmts.push(assign_attr(Attr::Age, lit_f(0.0)));
    let spawn = TBlock {
        kind: BlockKind::Spawn,
        span: span(),
        stmts: spawn_stmts,
    };

    let more_adversarial_lets = [
        "t",
        "dt",
        "capacity",
        "range",
        "survivors",
        "spawn_count",
        "seed_base",
        "mod_f32",
        "alive",
    ];
    let mut element_stmts: Vec<TStmt> = more_adversarial_lets
        .iter()
        .enumerate()
        .map(|(n, name)| let_(name, lit_f(n as f32)))
        .collect();
    element_stmts.push(assign_attr(
        Attr::Age,
        bin(BinOp::Rem, attr(Attr::Age), lit_f(1.0), Ty::Float),
    ));
    element_stmts.push(assign_attr(
        Attr::Position,
        bin(
            BinOp::Add,
            attr(Attr::Position),
            call(Builtin::Curl, vec![attr(Attr::Position)], Ty::Vec3),
            Ty::Vec3,
        ),
    ));
    element_stmts.push(TStmt::If {
        cond: bin(BinOp::Gt, attr(Attr::Age), lit_f(1.0), Ty::Bool),
        then: vec![TStmt::Kill { span: span() }],
        els: vec![],
        span: span(),
    });
    let element = TBlock {
        kind: BlockKind::Element,
        span: span(),
        stmts: element_stmts,
    };

    Checked {
        name: "shadowing_locals_l1".to_string(),
        kind: Kind::L1,
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: None,
        params: vec![param_decl("radius", Ty::Float, 0.1, 8.0)],
        emit: vec![Attr::Position, Attr::Age],
        consumes: vec![],
        blocks: vec![spawn, element],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// The L4 counterpart: `let`s named after `vertex`/`fragment`'s bare
/// identifiers — the uniform binding (`u`), the storage bindings
/// (`elements`, `alive`), the vertex/fragment builtin parameter names
/// (`elem`, `in`, `out`, `corner`, `corner_idx`), the storage buffer name
/// for a consumed attribute (`attr_position`), the fragment-only ambient
/// (`point_coord`), and two helper function names (`hash1`, `hsv_to_rgb`,
/// `corner_of`).
fn shadowing_locals_l4() -> Checked {
    let vertex_adversarial = [
        "u",
        "seed",
        "elem",
        "in",
        "out",
        "corner",
        "corner_idx",
        "attr_position",
        "elements",
        "alive",
    ];
    let mut vertex_stmts: Vec<TStmt> = vertex_adversarial
        .iter()
        .enumerate()
        .map(|(n, name)| let_(name, lit_f(n as f32)))
        .collect();
    vertex_stmts.push(assign_output(
        Output::Clip,
        bin(
            BinOp::Mul,
            ambient(Ambient::Camera, Ty::Mat4),
            construct(Ty::Vec4, vec![attr(Attr::Position), lit_f(1.0)]),
            Ty::Vec4,
        ),
    ));
    vertex_stmts.push(assign_output(
        Output::PointSize,
        param("point_scale", Ty::Float),
    ));
    let vertex = TBlock {
        kind: BlockKind::Vertex,
        span: span(),
        stmts: vertex_stmts,
    };

    let fragment_adversarial = ["point_coord", "hash1", "hsv_to_rgb", "corner_of"];
    let mut fragment_stmts: Vec<TStmt> = fragment_adversarial
        .iter()
        .enumerate()
        .map(|(n, name)| let_(name, lit_f(n as f32)))
        .collect();
    fragment_stmts.push(let_(
        "c",
        call(
            Builtin::HsvToRgb,
            vec![construct(
                Ty::Vec3,
                vec![param("hue", Ty::Float), lit_f(0.7), lit_f(1.0)],
            )],
            Ty::Vec3,
        ),
    ));
    fragment_stmts.push(let_(
        "a",
        call(
            Builtin::Length,
            vec![ambient(Ambient::PointCoord, Ty::Vec2)],
            Ty::Float,
        ),
    ));
    fragment_stmts.push(assign_output(
        Output::Color,
        construct(Ty::Vec4, vec![local("c", Ty::Vec3), local("a", Ty::Float)]),
    ));
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: fragment_stmts,
    };

    Checked {
        name: "shadowing_locals_l4".to_string(),
        kind: Kind::L4,
        // Not `None`: an L4's topology is inferred by the check pass, and
        // `generate_l4` reads it to choose the quad expansion. These are
        // hand-built stand-ins for checked trees, so they carry what `check`
        // would have put here.
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: Some(Blend::Additive),
        params: vec![
            param_decl("point_scale", Ty::Float, 0.5, 40.0),
            param_decl("hue", Ty::Float, 0.0, 1.0),
        ],
        emit: vec![],
        consumes: vec![Attr::Position],
        blocks: vec![vertex, fragment],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// An L1 procedure whose `param`s are named after WGSL reserved words that
/// are ordinary, unremarkable identifiers in `.kir` — `array` is the one
/// that was actually caught reaching a real GPU (see the bug report this
/// test locks in), the rest are here because WGSL reserves a great many
/// more than IR does and a generator has no reason to avoid any of them.
/// Every one of these is exactly the kind of word a procedure *about*
/// something would reach for: a particle `array`, a `loop` count, a `switch`
/// threshold.
fn reserved_word_params_l1() -> Checked {
    let names = [
        "array", "struct", "loop", "switch", "fn", "discard", "const", "override", "ptr", "sampler",
    ];
    let params = names
        .iter()
        .map(|n| param_decl(n, Ty::Float, 0.0, 1.0))
        .collect();

    let sum = names
        .iter()
        .map(|n| param(n, Ty::Float))
        .reduce(|acc, p| bin(BinOp::Add, acc, p, Ty::Float))
        .expect("at least one reserved-word param");
    let element = TBlock {
        kind: BlockKind::Element,
        span: span(),
        stmts: vec![assign_attr(Attr::Age, sum)],
    };

    Checked {
        name: "reserved_word_params".to_string(),
        kind: Kind::L1,
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: None,
        params,
        emit: vec![Attr::Age],
        consumes: vec![],
        blocks: vec![element],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// Every field `layout` declares must appear in `source` spelled exactly
/// `wgsl_name`, and `karakuri-engine`'s uniform packer keys its lookups on
/// `name` — so this also pins the two names apart: `name` must survive
/// unmangled (Set records address a param by its declared `.kir` name, and
/// the packer's lookups have to match that), while `wgsl_name` is what
/// actually appears in the WGSL text.
fn assert_layout_matches_text(source: &str, layout: &karakuri_codegen::layout::UniformLayout) {
    for f in &layout.fields {
        let decl = format!("{}: {},", f.wgsl_name, f.wgsl_ty);
        assert!(
            source.contains(&decl),
            "field {:?} (wgsl_name {:?}) is declared in the layout but not found in the emitted struct as {decl:?}:\n{source}",
            f.name,
            f.wgsl_name,
        );
    }
}

/// `generate_l4` takes its paired L1's `ElementLayout` rather than deriving
/// one from `consumes` (see that function's doc) — every fixture below is
/// built so its `consumes` is a subset of some L1 fixture's `emit`, and this
/// derives the layout that L1 side would have produced.
fn layout_for(l1: &Checked) -> karakuri_ir::layout::ElementLayout {
    karakuri_ir::layout::generate_element_layout(
        &l1.emit,
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    )
}

fn validate(source: &str) {
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|e| {
        panic!(
            "WGSL failed to parse:\n{}\n\n---- source ----\n{source}",
            e.emit_to_string(source)
        )
    });
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("WGSL failed validation: {e}\n\n---- source ----\n{source}"));
}

#[test]
fn drift_shell_l1_compiles_and_validates() {
    let shader = karakuri_codegen::generate_l1(&drift_shell(), &[], &[]);
    validate(&shader.source);
}

#[test]
fn soft_points_l4_compiles_and_validates() {
    let shader = karakuri_codegen::generate_l4(&soft_points(), &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// `param array : float ...` parses, checks, and costs — WGSL reserves
/// `array` but `.kir` does not — and before the fix this generated a struct
/// field literally named `array`, which naga rejects as a reserved keyword.
/// Every param name in this fixture is a real WGSL reserved word.
#[test]
fn params_named_after_wgsl_reserved_words_compile_and_validate() {
    let shader = karakuri_codegen::generate_l1(&reserved_word_params_l1(), &[], &[]);
    validate(&shader.source);
}

/// The layout this crate publishes and the WGSL text it emits must never
/// disagree about a field's name — `karakuri-engine`'s uniform packer
/// writes bytes at the offset `UniformLayout` gives it, on the assumption
/// that the struct in the WGSL text has a field at that same offset under
/// the name it looked up. A mismatch here would not fail loudly in this
/// crate; it would fail as a panic in `UniformPacker`, or worse, silently
/// pack a value into the wrong param's bytes.
#[test]
fn uniform_layout_and_emitted_struct_text_agree() {
    let l1 = karakuri_codegen::generate_l1(&drift_shell(), &[], &[]);
    assert_layout_matches_text(&l1.source, &l1.uniform_layout);

    let l4 = karakuri_codegen::generate_l4(&soft_points(), &layout_for(&drift_shell()), &[]);
    assert_layout_matches_text(&l4.source, &l4.uniform_layout);

    let reserved = karakuri_codegen::generate_l1(&reserved_word_params_l1(), &[], &[]);
    assert_layout_matches_text(&reserved.source, &reserved.uniform_layout);

    // And specifically: the layout's semantic `name` must stay the
    // undecorated param name (what a Set record and the engine's packer
    // both address it by), even though the WGSL text spells it mangled.
    let array_field = reserved
        .uniform_layout
        .fields
        .iter()
        .find(|f| f.name == "array")
        .expect("the `array` param must be in the layout under its declared name");
    assert_eq!(array_field.wgsl_name, "param_array");
}

/// The regression this whole file exists for: a local named `u` (or `seed`,
/// or `hash1`, or any of this crate's other fixed identifiers) must not be
/// capturable. Before the fix, this failed exactly the way the bug report
/// describes — naga rejecting a `u.<param>` access because a same-named
/// local shadowed the uniform binding.
#[test]
fn l1_locals_cannot_shadow_generated_identifiers() {
    let shader = karakuri_codegen::generate_l1(&shadowing_locals_l1(), &[], &[]);
    validate(&shader.source);
}

#[test]
fn l4_locals_cannot_shadow_generated_identifiers() {
    let shader = karakuri_codegen::generate_l4(
        &shadowing_locals_l4(),
        &layout_for(&shadowing_locals_l1()),
        &[],
    );
    validate(&shader.source);
}

/// An L4 that consumes a strict subset of `drift_shell`'s `emit` and names
/// them in a different order than `drift_shell` declares (`emit position,
/// velocity, age`; this consumes `age` then `position`, skipping
/// `velocity`). Attribute reads are resolved by field name
/// (`elements[elem].<name>`), never by struct position, so `generate_l4`
/// must not care about `consumes`' order or completeness relative to
/// `emit` — only about which names it happens to read.
fn reordered_subset_l4() -> Checked {
    let clip = bin(
        BinOp::Mul,
        ambient(Ambient::Camera, Ty::Mat4),
        construct(Ty::Vec4, vec![attr(Attr::Position), lit_f(1.0)]),
        Ty::Vec4,
    );
    let vertex = TBlock {
        kind: BlockKind::Vertex,
        span: span(),
        stmts: vec![
            assign_output(Output::Clip, clip),
            assign_output(
                Output::PointSize,
                bin(BinOp::Add, attr(Attr::Age), lit_f(1.0), Ty::Float),
            ),
        ],
    };
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: vec![assign_output(
            Output::Color,
            construct(Ty::Vec4, vec![lit_f(1.0)]),
        )],
    };

    Checked {
        name: "reordered_subset_l4".to_string(),
        kind: Kind::L4,
        // Not `None`: an L4's topology is inferred by the check pass, and
        // `generate_l4` reads it to choose the quad expansion. These are
        // hand-built stand-ins for checked trees, so they carry what `check`
        // would have put here.
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: Some(Blend::Additive),
        params: vec![],
        emit: vec![],
        consumes: vec![Attr::Age, Attr::Position],
        blocks: vec![vertex, fragment],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// An L4 that consumes nothing at all — legal per the ir-spec ("the
/// `soft_points` example ... consumes three attributes and emits nothing,
/// which is not an error but the normal shape of an L4 file"; the reverse,
/// consuming nothing, is equally legal and untested elsewhere in this
/// file). It still declares the full `Element` struct L1 wrote and still
/// reads `seed`, just no named attribute.
fn consumes_nothing_l4() -> Checked {
    let vertex = TBlock {
        kind: BlockKind::Vertex,
        span: span(),
        stmts: vec![
            assign_output(
                Output::Clip,
                construct(
                    Ty::Vec4,
                    vec![lit_f(0.0), lit_f(0.0), lit_f(0.0), lit_f(1.0)],
                ),
            ),
            assign_output(Output::PointSize, lit_f(4.0)),
        ],
    };
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: vec![assign_output(
            Output::Color,
            construct(
                Ty::Vec4,
                vec![call(
                    Builtin::Hash1,
                    vec![ambient(Ambient::Seed, Ty::Uint)],
                    Ty::Float,
                )],
            ),
        )],
    };

    Checked {
        name: "consumes_nothing_l4".to_string(),
        kind: Kind::L4,
        // Not `None`: an L4's topology is inferred by the check pass, and
        // `generate_l4` reads it to choose the quad expansion. These are
        // hand-built stand-ins for checked trees, so they carry what `check`
        // would have put here.
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: Some(Blend::Additive),
        params: vec![],
        emit: vec![],
        consumes: vec![],
        blocks: vec![vertex, fragment],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// `consumes` names a strict subset of `emit`, out of declaration order.
/// This is the case the module doc on `generate_l4` calls out by name:
/// byte identity must hold regardless of what a paired L4 happens to read.
#[test]
fn l4_consuming_a_reordered_subset_compiles_and_validates() {
    let shader =
        karakuri_codegen::generate_l4(&reordered_subset_l4(), &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// `consumes` is empty. Legal per the ir-spec; the mirror image of
/// `soft_points`, which emits nothing.
#[test]
fn l4_consuming_nothing_compiles_and_validates() {
    let shader =
        karakuri_codegen::generate_l4(&consumes_nothing_l4(), &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// An L1 procedure that emits every declarable attribute (all seven of
/// `Attr::ALL`), the shape this whole change exists for: before it, this
/// would have bound 2 synthetic + 7 declared = 9 attributes, times prev and
/// next, an 18-buffer compute stage. The `Element` struct is 9 `vec4`
/// slots (`seed`, `birth_frac`, plus the 7 declared) regardless, and the
/// bind group stays 2 storage buffers per direction.
fn emits_every_attribute_l1() -> Checked {
    let element = TBlock {
        kind: BlockKind::Element,
        span: span(),
        stmts: karakuri_ir::Attr::ALL
            .into_iter()
            .map(|a| assign_attr(a, zero_expr(a.ty())))
            .collect(),
    };

    Checked {
        name: "emits_every_attribute".to_string(),
        kind: Kind::L1,
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: None,
        params: vec![],
        emit: karakuri_ir::Attr::ALL.to_vec(),
        consumes: vec![],
        blocks: vec![element],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// A zero value of `ty`, using the same broadcast-scalar vector constructor
/// the ir-spec documents (`vec3(0.0)`), since `Lit` itself only carries
/// scalars.
fn zero_expr(ty: Ty) -> TExpr {
    match ty {
        Ty::Float => lit_f(0.0),
        Ty::Vec2 | Ty::Vec3 => construct(ty, vec![lit_f(0.0)]),
        other => panic!("zero_expr does not cover {other:?}; no attribute needs it"),
    }
}

/// The buffer-count claim this whole change exists for: the `Element`
/// struct's field count tracks `emit.len()` exactly (`seed`, `birth_frac`,
/// and every declared attribute), and it still validates as real WGSL at
/// the largest shape the language allows: all seven declarable attributes,
/// not just the two or three every other fixture in this file emits.
#[test]
fn l1_emitting_every_attribute_compiles_and_validates() {
    let checked = emits_every_attribute_l1();
    let shader = karakuri_codegen::generate_l1(&checked, &[], &[]);
    assert_eq!(
        shader.element_layout.slots.len(),
        2 + karakuri_ir::Attr::ALL.len()
    );
    // **Every attribute at once, which is the widest element the language can
    // ask for**, and the number is asserted rather than derived so that a
    // layout change has to come here and say what it did. The padded layout
    // made this `(2 + ALL) * 16`; WGSL's own placement makes it this.
    assert_eq!(shader.element_layout.stride, 96);
    // And it is smaller than the layout it replaced, which is the whole point.
    assert!(shader.element_layout.stride < (2 + karakuri_ir::Attr::ALL.len() as u32) * 16);
    validate(&shader.source);
}

/// The L4 side of the same claim, paired against the all-attributes L1:
/// consuming every attribute still produces a byte-identical `Element`
/// struct and valid WGSL.
#[test]
fn l4_consuming_every_attribute_compiles_and_validates() {
    let l1 = emits_every_attribute_l1();
    let clip = construct(Ty::Vec4, vec![attr(Attr::Position), lit_f(1.0)]);
    let vertex = TBlock {
        kind: BlockKind::Vertex,
        span: span(),
        stmts: vec![
            assign_output(Output::Clip, clip),
            assign_output(Output::PointSize, lit_f(1.0)),
        ],
    };
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: vec![assign_output(
            Output::Color,
            construct(Ty::Vec4, vec![lit_f(1.0)]),
        )],
    };
    let l4 = Checked {
        name: "consumes_every_attribute".to_string(),
        kind: Kind::L4,
        // Not `None`: an L4's topology is inferred by the check pass, and
        // `generate_l4` reads it to choose the quad expansion. These are
        // hand-built stand-ins for checked trees, so they carry what `check`
        // would have put here.
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: Some(Blend::Additive),
        params: vec![],
        emit: vec![],
        consumes: karakuri_ir::Attr::ALL.to_vec(),
        blocks: vec![vertex, fragment],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    };
    let shader = karakuri_codegen::generate_l4(&l4, &layout_for(&l1), &[]);
    validate(&shader.source);
}

/// The core claim the L4-takes-a-layout signature exists for: L1 and its
/// paired L4 must declare byte-identical `Element` structs, since L4 reads
/// the same physical buffer L1 wrote. Extracts the `struct Element { ... };`
/// block from each generated source and compares the text directly, rather
/// than trusting that "compiles" implies "same layout" — two structs with a
/// different field order would each compile fine on their own.
#[test]
fn l1_and_paired_l4_declare_byte_identical_element_structs() {
    fn element_struct_text(source: &str) -> &str {
        let start = source
            .find("struct Element {")
            .expect("no Element struct in source");
        let end = source[start..]
            .find("};")
            .expect("unterminated Element struct")
            + start
            + 2;
        &source[start..end]
    }

    let l1 = karakuri_codegen::generate_l1(&drift_shell(), &[], &[]);
    let l4 = karakuri_codegen::generate_l4(&soft_points(), &layout_for(&drift_shell()), &[]);
    assert_eq!(
        element_struct_text(&l1.source),
        element_struct_text(&l4.source),
        "L1:\n{}\n\nL4:\n{}",
        l1.source,
        l4.source
    );
}

/// The negative half of "a generator whose output is never fed to a
/// compiler emits plausible nonsense": confirm this test harness actually
/// catches a broken shader, not just that well-formed ones pass. The type
/// mismatch below is caught by validation, not by parsing — WGSL's grammar
/// alone does not know a function's declared return type disagrees with
/// what it returns, so this has to go through both stages, exactly like the
/// two real fixtures above do.
#[test]
fn naga_rejects_a_deliberately_broken_shader() {
    let broken = "\
@fragment
fn fs() -> @location(0) f32 {
    return vec3<f32>(1.0, 2.0, 3.0);
}
";
    let module =
        naga::front::wgsl::parse_str(broken).expect("this fixture is syntactically valid WGSL");
    let result = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module);
    assert!(
        result.is_err(),
        "expected a return-type mismatch to fail validation"
    );
}

/// `soft_points` with a second endpoint: the same procedure, made a line
/// renderer by the one assignment that decides it.
///
/// Deliberately a *modification* of the points fixture rather than a fresh
/// tree, so the difference between the two generated shaders is the
/// difference between the two topologies and nothing else.
fn soft_streaks() -> Checked {
    let mut checked = soft_points();
    checked.name = "soft_streaks".to_string();
    checked.topology = Some(Topology::Lines);
    let tail = bin(
        BinOp::Mul,
        ambient(Ambient::Camera, Ty::Mat4),
        construct(
            Ty::Vec4,
            vec![
                bin(
                    BinOp::Sub,
                    attr(Attr::Position),
                    attr(Attr::Velocity),
                    Ty::Vec3,
                ),
                lit_f(1.0),
            ],
        ),
        Ty::Vec4,
    );
    let vertex = checked
        .blocks
        .iter_mut()
        .find(|b| b.kind == BlockKind::Vertex)
        .expect("the points fixture has a vertex block");
    vertex.stmts.push(assign_output(Output::ClipB, tail));
    checked
}

/// The segment expansion has to be WGSL a front end accepts, not merely text
/// that reads correctly. It uses `mix` on vectors, a swizzle-built
/// perpendicular, and a division by an interpolated `w` — three things that
/// are easy to write and easy to get past a reviewer while naga refuses them.
#[test]
fn a_lines_l4_compiles_and_validates() {
    let shader = karakuri_codegen::generate_l4(&soft_streaks(), &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// Each topology emits **its own** placement, and neither emits the other's.
///
/// The first version of this asserted only that the two sources differed,
/// which they do for a reason that has nothing to do with the expansion: a
/// lines procedure declares `_clip_b` and assigns it in the body whatever the
/// generator then does with it. A generator that ignored the topology and ran
/// the point path twice passed. So the assertion names the two placements
/// instead — `ndc_offset`, the offset around a point, and `half_vp`, the pixel
/// conversion only the segment expansion needs — and requires each shader to
/// have exactly one of them.
#[test]
fn each_topology_emits_its_own_placement_and_not_the_others() {
    let layout = layout_for(&drift_shell());
    let points = karakuri_codegen::generate_l4(&soft_points(), &layout, &[]).source;
    let lines = karakuri_codegen::generate_l4(&soft_streaks(), &layout, &[]).source;

    assert!(
        points.contains("ndc_offset"),
        "the points shader lost the sprite offset"
    );
    assert!(
        !points.contains("half_vp"),
        "the points shader is expanding a segment"
    );
    assert!(
        lines.contains("half_vp"),
        "the lines shader is not expanding a segment"
    );
    assert!(
        !lines.contains("ndc_offset"),
        "the lines shader is still offsetting around a point"
    );

    // The fragment stage is topology-blind: same entry point, same varyings.
    let fs = |s: &str| s[s.find("@fragment").expect("a fragment stage")..].to_string();
    assert_eq!(
        fs(&points),
        fs(&lines),
        "the topology reached the fragment stage"
    );
}

/// `soft_points` under the other blend mode. A *modification* of the points
/// fixture for the same reason `soft_streaks` is: what differs between the two
/// generated shaders is then the blend mode and nothing else.
fn soft_glass() -> Checked {
    let mut checked = soft_points();
    checked.name = "soft_glass".to_string();
    checked.blend = Some(Blend::Weighted);
    checked
}

/// The weighted fragment stage is two targets, a clamp, a `pow` and a divide
/// that has not happened yet — none of which a reviewer can confirm is WGSL a
/// front end accepts. A struct-returning `@fragment` with a scalar at
/// `@location(1)` is the part most likely to be almost right.
#[test]
fn a_weighted_l4_compiles_and_validates() {
    let shader = karakuri_codegen::generate_l4(&soft_glass(), &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// And the same for segments, because the depth a weighted stroke is weighted
/// by comes out of the segment expansion rather than straight off `clip`.
#[test]
fn a_weighted_lines_l4_compiles_and_validates() {
    let mut checked = soft_streaks();
    checked.blend = Some(Blend::Weighted);
    let shader = karakuri_codegen::generate_l4(&checked, &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// A weighted L4 with no `vertex` block. **`Set::build` refuses this pairing**
/// — one layer per texel makes the resolve the identity — so nothing in the
/// engine's tests can reach the code that lowers it, and without this the
/// generator's fullscreen-plus-weighted arm would be the one path in the crate
/// no front end had ever read.
#[test]
fn a_weighted_fullscreen_l4_compiles_and_validates() {
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: vec![assign_output(
            Output::Color,
            construct(Ty::Vec4, vec![ambient(Ambient::Ray, Ty::Vec3), lit_f(0.5)]),
        )],
    };
    let l4 = Checked {
        name: "weighted_march".to_string(),
        kind: Kind::L4,
        topology: Some(Topology::Fullscreen),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: Some(Blend::Weighted),
        params: vec![],
        emit: vec![],
        consumes: vec![],
        blocks: vec![fragment],
        cost: None,
        closed_form: false,
        reads_beats: false,
        span: span(),
    };
    let shader = karakuri_codegen::generate_l4(&l4, &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
    assert!(
        !shader
            .uniform_layout
            .fields
            .iter()
            .any(|f| f.name == "depth_range"),
        "a frame is not at a depth, so there is nothing for a range to normalise it against"
    );
}

/// **Each blend mode emits its own fragment epilogue, and neither emits the
/// other's** — the same shape of assertion as
/// `each_topology_emits_its_own_placement_and_not_the_others`, and written after
/// that one caught a generator ignoring the topology it was handed.
///
/// The three things named are the three that have to move together: the second
/// target, the varying the weight is computed from, and the camera planes that
/// varying is measured against. A shader with any one of them missing would
/// still compile.
///
/// **The planes are a read and not a declaration.** They live in the camera's
/// bind group, which an additive shader binds too — it projects with the same
/// camera — so what separates the two modes is `cam.depth_range` appearing in
/// the body, not `depth_range` appearing anywhere. Asserting on the name alone
/// would pass for both, since both emit the struct that declares it.
#[test]
fn each_blend_mode_emits_its_own_fragment_epilogue_and_not_the_others() {
    let layout = layout_for(&drift_shell());
    let additive = karakuri_codegen::generate_l4(&soft_points(), &layout, &[]);
    let weighted = karakuri_codegen::generate_l4(&soft_glass(), &layout, &[]);

    assert!(
        weighted.source.contains("@location(1) reveal"),
        "no revealage target"
    );
    assert!(
        weighted.source.contains("view_depth"),
        "no depth to weigh by"
    );
    assert!(
        weighted.source.contains("cam.depth_range"),
        "no camera planes to measure the depth against"
    );
    assert!(
        weighted.camera_group.is_some(),
        "nothing to read those planes from"
    );

    // Not `@location(1)`: an additive shader has one of those already, for its
    // first varying. What it must not have is a fragment output struct.
    assert!(
        !additive.source.contains("FsOut"),
        "the additive shader grew a second target"
    );
    assert!(
        !additive.source.contains("reveal"),
        "the additive shader accumulates revealage"
    );
    assert!(
        !additive.source.contains("view_depth"),
        "the additive shader carries a depth nothing reads"
    );
    assert!(
        !additive.source.contains("cam.depth_range"),
        "the additive shader weighs a fragment by its depth"
    );

    // The vertex stage's *placement* is blend-blind: the same six corners land
    // in the same six places whichever way the fragments are combined.
    let placement = |s: &str| {
        let vs = &s[s.find("@vertex").expect("a vertex stage")..];
        vs[..vs.find("@fragment").expect("a fragment stage")]
            .lines()
            .filter(|l| !l.contains("view_depth"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        placement(&additive.source),
        placement(&weighted.source),
        "the blend mode moved the geometry"
    );
}

// ---------------------------------------------------------------------------
// L3 — the camera
// ---------------------------------------------------------------------------

/// A sweep round the origin: `eye` from the clock and a param, `target` at the
/// centre, and the other four left to their defaults.
fn sweep() -> Checked {
    let angle = bin(
        BinOp::Mul,
        bin(
            BinOp::Mul,
            ambient(Ambient::T, Ty::Float),
            param("speed", Ty::Float),
            Ty::Float,
        ),
        lit_f(std::f32::consts::TAU),
        Ty::Float,
    );
    let on_circle = |f: Builtin| {
        bin(
            BinOp::Mul,
            call(f, vec![local("a", Ty::Float)], Ty::Float),
            param("radius", Ty::Float),
            Ty::Float,
        )
    };
    let camera = TBlock {
        kind: BlockKind::Camera,
        span: span(),
        stmts: vec![
            let_("a", angle),
            assign_output(
                Output::Eye,
                construct(
                    Ty::Vec3,
                    vec![on_circle(Builtin::Cos), lit_f(2.0), on_circle(Builtin::Sin)],
                ),
            ),
            assign_output(Output::Target, construct(Ty::Vec3, vec![lit_f(0.0)])),
        ],
    };

    Checked {
        name: "sweep".to_string(),
        kind: Kind::L3,
        topology: None,
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: None,
        params: vec![
            param_decl("radius", Ty::Float, 1.0, 40.0),
            param_decl("speed", Ty::Float, 0.0, 2.0),
        ],
        emit: vec![],
        consumes: vec![],
        blocks: vec![camera],
        cost: None,
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

#[test]
fn a_camera_shader_parses_and_validates() {
    validate(&karakuri_codegen::generate_l3(&sweep(), &[]).source);
}

/// **The four an author did not write are written anyway**, before the block
/// runs — which is what makes them defaults rather than requirements, and what
/// keeps the simplest camera anyone writes four lines shorter.
///
/// **The values are checked, not only their presence.** Zeroing them leaves a
/// shader that still writes all six and still validates — and produces a camera
/// with no field of view, no depth range and no up vector, which is a blank
/// frame with no diagnostic. Defect injection found exactly that hole in an
/// earlier version of this test.
///
/// The numbers are `Orbit::default`'s, so replacing the built-in camera with an
/// L3 does not quietly change the field of view underneath the picture. That
/// they *are* `Orbit::default`'s is checked in `karakuri-engine`, where both
/// exist; here they are held against the values that struct documents.
#[test]
fn every_camera_output_is_written_whether_or_not_the_block_assigned_it() {
    let src = karakuri_codegen::generate_l3(&sweep(), &[]).source;
    for field in ["eye", "look_at", "up", "fov_y", "near", "far"] {
        assert!(
            src.contains(&format!("cam.{field}")),
            "`{field}` never reaches the state buffer:\n{src}"
        );
    }
    // And the block's own assignment lands on the local, not on the buffer, so
    // a block that writes `eye` twice writes the buffer once.
    assert_eq!(
        src.matches("cam.eye").count(),
        1,
        "the eye is stored more than once:\n{src}"
    );
    assert!(
        src.contains("_eye ="),
        "the block's assignment did not reach a local:\n{src}"
    );

    // A third of pi, and 0.1 to 100 — see `karakuri_engine::camera::Orbit`'s
    // `Default`. `up` is `+y`, which is the only one of the four that a wrong
    // value would show as a picture rather than as no picture.
    assert!(
        src.contains("_up     = vec3<f32>(0.0, 1.0, 0.0)"),
        "up is not +y:\n{src}"
    );
    assert!(
        src.contains("_fov_y  = 1.0471976"),
        "the field of view is not a third of pi:\n{src}"
    );
    assert!(src.contains("_near   = 0.1"), "{src}");
    assert!(src.contains("_far    = 100.0"), "{src}");
}

/// **One invocation, and no index.** A camera block runs once a frame and
/// produces six numbers — there is nothing to be at index `i` of, so a bounds
/// check or an alive flag here would be a transcription of another layer's
/// entry point rather than this one's.
#[test]
fn a_camera_shader_dispatches_one_invocation_over_nothing() {
    let src = karakuri_codegen::generate_l3(&sweep(), &[]).source;
    assert!(src.contains("@workgroup_size(1)"), "{src}");
    assert!(
        !src.contains("global_invocation_id"),
        "a camera indexed something:\n{src}"
    );
    assert!(
        !src.contains("counts"),
        "a camera read a live range:\n{src}"
    );
}

/// The uniform carries the clock and the params, and nothing per element.
#[test]
fn a_camera_uniform_carries_the_clock_and_its_params() {
    let shader = karakuri_codegen::generate_l3(&sweep(), &[]);
    let names: Vec<&str> = shader
        .uniform_layout
        .fields
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["t", "beats", "dt", "seed_salt", "radius", "speed"]
    );
}

/// **Two blocks, one function, and a local name may appear in both.**
///
/// `mask` and `deform` are spliced into one WGSL entry point and a local is
/// mangled by its name alone, so `let d` in each would be a redefinition — from
/// a `.kir` the checker accepted, since it checks each block in its own scope.
/// The mask's statements get a WGSL scope to make that true, and this is the
/// test that would have caught the shader failing to compile at build time.
///
/// The names are not exotic: `examples/late_bloom.kir`'s `deform` declares `out`
/// and `away`, and a mask written against the same geometry reaches for the same
/// words.
#[test]
fn a_mask_and_a_deform_may_declare_the_same_local() {
    let src = r#"
proc collides {
  kind L2
  consumes position, age
  mask   { let d = length(position); strength = d; }
  deform { let d = age * 2.0; position = position * d; }
}
"#;
    let parsed = karakuri_ir::parse(src).expect("parses");
    let checked = karakuri_ir::check::check(&parsed).expect("checks");
    let shader = karakuri_codegen::generate_l2(
        &checked,
        &[Attr::Position, Attr::Age],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
        None,
        &[],
    );
    validate(&shader.source);
}

// ---------------------------------------------------------------------------
// Amplification: an L2 whose output count differs from its input's.
// ---------------------------------------------------------------------------

fn compiled_l2(
    src: &str,
    upstream: &[Attr],
    synthetic: karakuri_ir::layout::Synthetic,
) -> karakuri_codegen::L2Shader {
    let parsed = karakuri_ir::parse(src).expect("parses");
    let checked = karakuri_ir::check::check(&parsed).expect("checks");
    karakuri_codegen::generate_l2(&checked, upstream, synthetic, &[], None, &[])
}

const MIRROR: &str = r#"
proc mirror {
  kind    L2
  amplify 4
  consumes position
  deform { position = position + vec3(0.0, float(copy), 0.0); }
}
"#;

/// The amplifying entry point is a different shape from the endomorphic one —
/// a loop over the copies, a second element index, and a liveness buffer of its
/// own — so it gets its own trip through naga.
#[test]
fn an_amplifying_l2_lowers_to_valid_wgsl() {
    let shader = compiled_l2(
        MIRROR,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    validate(&shader.source);
    assert_eq!(shader.amplify, Some(4));
    assert!(
        shader.synthetic.copy,
        "the node that amplified is where `copy` starts existing"
    );
    assert!(
        shader.element_layout.slots.iter().any(|s| s.name == "copy"),
        "the output carries the index: {:?}",
        shader.element_layout
    );
}

/// **An amplifier writes liveness and an endomorphism does not**, because an
/// endomorphism shares the very buffer its input came with and an amplifier
/// cannot — its own is `factor` times as long. The binding's presence is the
/// observable half of that.
#[test]
fn only_an_amplifying_l2_binds_a_liveness_buffer_to_write() {
    let amplifying = compiled_l2(
        MIRROR,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    assert!(
        amplifying.source.contains("dst_alive"),
        "{}",
        amplifying.source
    );

    let plain = compiled_l2(
        r#"
proc plain {
  kind L2
  consumes position
  deform { position = position * 2.0; }
}
"#,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    assert!(!plain.source.contains("dst_alive"), "{}", plain.source);
}

/// **Stacked amplifiers compose the index rather than overwrite it.** A node of
/// factor `n` under a parent that already carried a `copy` writes
/// `copy * n + c`, which is the mixed-radix numbering of the whole chain: it
/// stays unique, and the parent's index is still recoverable by dividing.
/// Overwriting would make two elements of one parent indistinguishable the
/// moment a second amplifier ran, which is the whole of what `copy` is for.
#[test]
fn a_second_amplifier_composes_the_copy_index_rather_than_replacing_it() {
    // The first has nothing above it, so it starts the numbering from the
    // loop variable alone.
    let first = compiled_l2(
        MIRROR,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    assert!(
        first.source.contains("dst[i].copy = 0u * 4u + _c;"),
        "a first amplifier numbers from nothing: {}",
        first.source
    );

    // The second is handed the first's output, and reads the parent's index.
    let second = compiled_l2(
        r#"
proc again {
  kind    L2
  amplify 3
  consumes position
  deform { position = position * 2.0; }
}
"#,
        &[Attr::Position],
        first.synthetic,
    );
    validate(&second.source);
    assert!(
        second
            .source
            .contains("dst[i].copy = src[_e].copy * 3u + _c;"),
        "a second amplifier composes: {}",
        second.source
    );
}

/// A node below an amplifier that does not amplify itself carries the index
/// through untouched — it is somebody else's identity, and passing it on is the
/// same pass-through every other slot gets.
#[test]
fn a_plain_l2_below_an_amplifier_carries_the_copy_index_through() {
    let plain = compiled_l2(
        r#"
proc plain {
  kind L2
  consumes position
  deform { position = position + vec3(0.0, float(copy), 0.0); }
}
"#,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic { copy: true },
    );
    validate(&plain.source);
    assert!(
        plain.source.contains("dst[i].copy = src[i].copy;"),
        "{}",
        plain.source
    );
    assert!(
        !plain.source.contains("dst_alive"),
        "it did not amplify: {}",
        plain.source
    );
}

/// Where nothing upstream amplified, `copy` is `0u` rather than a read of a
/// slot that is not there — the answer a chain with no amplifier in it gives at
/// every position in it.
#[test]
fn copy_reads_zero_where_no_slot_exists() {
    let plain = compiled_l2(
        r#"
proc plain {
  kind L2
  consumes position
  deform { position = position + vec3(0.0, float(copy), 0.0); }
}
"#,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    validate(&plain.source);
    assert!(plain.source.contains("f32(0u)"), "{}", plain.source);
}

/// A mask over an amplifying node has to end up inside the copy loop, because
/// what it gates is one copy rather than one parent — and the whole thing still
/// has to be WGSL naga accepts, which is what a spliced block in a nested scope
/// is easiest to get wrong.
#[test]
fn an_amplifying_l2_with_a_mask_lowers_to_valid_wgsl() {
    let shader = compiled_l2(
        r#"
proc masked {
  kind    L2
  amplify 4
  param   weight : float [0.0, 1.0] = 1.0
  consumes position
  mask   { let d = length(position); strength = d; }
  deform { let d = float(copy); position = position + vec3(0.0, d, 0.0); }
}
"#,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    validate(&shader.source);
}

/// **An L4 reads `copy` the way it reads `seed`** — off the element in the
/// vertex stage, carried to the fragment as a flat varying. It is the other
/// half of identity below an amplifier, and a renderer is downstream.
#[test]
fn an_l4_reading_copy_lowers_to_valid_wgsl() {
    let src = r#"
proc tinted {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    let h = hash1(seed + copy * 8191u);
    color   = vec4(h, h, h, 1.0);
  }
}
"#;
    let parsed = karakuri_ir::parse(src).expect("parses");
    let checked = karakuri_ir::check::check(&parsed).expect("checks");
    let amplified = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic { copy: true },
        &[],
    );
    let shader = karakuri_codegen::generate_l4(&checked, &amplified, &[]);
    validate(&shader.source);
    assert!(
        shader.source.contains("let copy = elements[elem].copy;"),
        "{}",
        shader.source
    );
    assert!(
        shader.source.contains("@interpolate(flat) copy: u32"),
        "{}",
        shader.source
    );
}

/// **The same L4 over geometry no amplifier touched still compiles**, and reads
/// zero. A renderer is compiled against whatever chain it was given and cannot
/// know whether one had an amplifier in it, so the answer where the slot is
/// absent has to be a value rather than a refusal — and copy zero of itself is
/// the true one.
#[test]
fn an_l4_reading_copy_over_unamplified_geometry_reads_zero() {
    let src = r#"
proc tinted {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    let h = hash1(copy);
    color   = vec4(h, h, h, 1.0);
  }
}
"#;
    let parsed = karakuri_ir::parse(src).expect("parses");
    let checked = karakuri_ir::check::check(&parsed).expect("checks");
    let plain = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );
    let shader = karakuri_codegen::generate_l4(&checked, &plain, &[]);
    validate(&shader.source);
    assert!(
        shader.source.contains("let copy = 0u;"),
        "{}",
        shader.source
    );
    assert!(
        !shader.source.contains("elements[elem].copy"),
        "{}",
        shader.source
    );
}

// ---------------------------------------------------------------------------
// A spliced field, in each of the five modules that can carry one.
// ---------------------------------------------------------------------------

/// A field exercising everything a spliced body can reach: the clock, a param,
/// an SDF builtin, a `mod`, and `fbm` — which this crate unrolls in Rust, so its
/// requirement lands in the *field's* set and has to be absorbed by the caller.
const SPLICED: &str = r#"
proc wobble {
  kind Field

  param radius : float [0.1, 3.0] = 1.0

  field {
    let n = fbm(point, 3) * 0.1;
    let a = mod(point.x, 2.0);
    distance = sd_sphere(point, radius) + n + a * 0.0 + sin(t + beats) * 0.0;
  }
}
"#;

/// The field itself, not a splice of it: a generator is handed the `kind Field`
/// procedure now and splices it once per slot its caller declared, because the
/// function's name is the caller's name for it.
fn spliced() -> karakuri_ir::typed::Checked {
    let parsed = karakuri_ir::parse(SPLICED).expect("parses");
    karakuri_ir::check::check(&parsed).expect("checks")
}

fn compiled(src: &str) -> karakuri_ir::typed::Checked {
    let parsed = karakuri_ir::parse(src).expect("parses");
    karakuri_ir::check::check(&parsed).expect("checks")
}

/// **Every caller kind, through a real WGSL front end.**
///
/// The engine's own tests reach one shape — a fullscreen L4 — so three of the
/// five splice sites had no coverage at any level, and deleting the splice from
/// any of them left the whole workspace green. What breaks is not subtle: a
/// module that names `_field_shape_at` and does not define it.
///
/// The field above reads `t` and `beats`, which each caller spells its own way,
/// and calls `fbm`, whose unrolled `perlin` requirement belongs to the field and
/// has to reach the caller's prelude.
#[test]
fn a_spliced_field_validates_in_every_kind_of_caller() {
    let field = spliced();
    let layout = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );

    // L1, in both of its blocks.
    let l1 = compiled(
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 64] = 8

  uses shape : Field

  param spawn_rate : float [0.0, 100.0] = 10.0

  emit position

  spawn   { position = vec3(shape(vec3(0.0, 0.0, 0.0)), 0.0, 0.0); }
  element { position = position + vec3(0.0, shape(position), 0.0) * dt; }
}
"#,
    );
    validate(&karakuri_codegen::generate_l1(&l1, &[], &[("shape", &field)]).source);

    // L2, in both of its blocks.
    let l2 = compiled(
        r#"
proc warp {
  kind L2
  uses shape : Field
  consumes position
  mask   { strength = clamp(shape(position), 0.0, 1.0); }
  deform { position = position * (1.0 + shape(position) * 0.01); }
}
"#,
    );
    validate(
        &karakuri_codegen::generate_l2(
            &l2,
            &[Attr::Position],
            karakuri_ir::layout::Synthetic::NONE,
            &[],
            None,
            &[("shape", &field)],
        )
        .source,
    );

    // L3.
    let l3 = compiled(
        r#"
proc look {
  kind L3
  uses shape : Field
  camera {
    eye    = vec3(0.0, 0.0, 4.0 + shape(vec3(0.0, 0.0, 0.0)));
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#,
    );
    validate(&karakuri_codegen::generate_l3(&l3, &[("shape", &field)]).source);

    // L4 with a vertex block — per element, which is a different generator from
    // the fullscreen one below.
    let l4 = compiled(
        r#"
proc dots {
  kind  L4
  blend additive

  uses shape : Field

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 2.0 + shape(position) * 0.0;
  }

  fragment {
    let d = shape(vec3(0.0, 0.0, 0.0));
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
    validate(&karakuri_codegen::generate_l4(&l4, &layout, &[("shape", &field)]).source);

    // L4 with none — fullscreen.
    let full = compiled(
        r#"
proc marcher {
  kind  L4
  blend additive

  uses shape : Field

  fragment {
    let d = shape(eye + ray);
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
    validate(&karakuri_codegen::generate_l4(&full, &layout, &[("shape", &field)]).source);
}

/// **A caller that mentions no field carries none of it**, which is what keeps
/// a bad field body from taking down shaders with nothing to do with it — and
/// keeps every node in the Set from growing the field's params.
#[test]
fn a_caller_that_evaluates_no_field_is_not_spliced() {
    let field = spliced();
    let plain = compiled(
        r#"
proc plain {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 2.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#,
    );
    let layout = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );
    let src = karakuri_codegen::generate_l4(&plain, &layout, &[("shape", &field)]).source;
    validate(&src);
    assert!(
        !src.contains("_field_shape_at"),
        "the function is not here: {src}"
    );
    assert!(
        !src.contains("field_shape_radius"),
        "and neither are its params: {src}"
    );
}

/// **The clock is the caller's own spelling**, passed at the call site. An L1
/// reads `t` per substep from `step_args` where everything else reads `u` — one
/// spliced body cannot say both, and a body that assumed either produced a
/// module naming a field that module does not have.
#[test]
fn a_spliced_field_takes_the_clock_from_its_caller() {
    let field = spliced();
    let l1 = compiled(
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 64] = 8

  uses shape : Field

  emit position

  element { position = vec3(shape(position), 0.0, 0.0); }
}
"#,
    );
    let src = karakuri_codegen::generate_l1(&l1, &[], &[("shape", &field)]).source;
    assert!(
        src.contains("step_args.t"),
        "an L1 passes its per-substep clock: {src}"
    );

    let layout = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );
    let full = compiled(
        r#"
proc marcher {
  kind  L4
  blend additive

  uses shape : Field

  fragment {
    let d = shape(eye);
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
    let src = karakuri_codegen::generate_l4(&full, &layout, &[("shape", &field)]).source;
    assert!(
        src.contains("_field_shape_at(") && src.contains("u.t"),
        "and a renderer passes `u.t`: {src}"
    );
}

// ---------------------------------------------------------------------------
// The element layout, checked against a real WGSL front end's own arithmetic.
//
// **A wrong align/size table is not a compile error anywhere.** The host writes
// bytes at `ElementSlot::offset` and the shader reads them through the struct —
// so if the two disagree, nothing refuses to build and nothing logs: it is an
// element reading the middle of the element before it, which reaches a screen
// as plausible material with the wrong values in it.
//
// Validating the module cannot catch that on its own, and the claim that it
// could was wrong for a specific reason: `write_element_struct` emits no
// `@offset` attributes at all, so naga computes every offset from its own rules
// and can never visibly disagree with ours. It agrees with itself. What is
// needed is to ask naga what it computed and compare, which is what these do.
// ---------------------------------------------------------------------------

/// naga's own placement for one generated struct: each member's name and byte
/// offset in declaration order, and the stride it gives `array<name>`.
///
/// **The array stride rather than the struct's span**, because the stride is
/// the number the engine actually multiplies a capacity by. WGSL rounds a
/// struct's size up to its own alignment to get it, so the two agree — and the
/// caller asserts that they do, since a front end that disagreed with itself
/// there would make every other assertion here meaningless.
fn naga_placement(source: &str, name: &str) -> (Vec<(String, u32)>, u32, u32) {
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|e| {
        panic!(
            "WGSL failed to parse:\n{}\n\n---- source ----\n{source}",
            e.emit_to_string(source)
        )
    });
    let (handle, members, span) = module
        .types
        .iter()
        .find_map(|(handle, ty)| match (&ty.name, &ty.inner) {
            (Some(n), naga::TypeInner::Struct { members, span }) if n == name => {
                Some((handle, members, *span))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no `struct {name}` in the emitted module:\n{source}"));
    let stride = module
        .types
        .iter()
        .find_map(|(_, ty)| match ty.inner {
            naga::TypeInner::Array { base, stride, .. } if base == handle => Some(stride),
            _ => None,
        })
        .unwrap_or_else(|| panic!("`{name}` is never bound as an array:\n{source}"));
    let placed = members
        .iter()
        .map(|m| {
            (
                m.name.clone().unwrap_or_else(|| "<unnamed>".to_string()),
                m.offset,
            )
        })
        .collect();
    (placed, span, stride)
}

/// Every slot of `layout`, at the name and offset naga put it at, and the
/// stride naga gives an array of it.
fn assert_naga_agrees(source: &str, name: &str, layout: &karakuri_ir::layout::ElementLayout) {
    let (placed, span, stride) = naga_placement(source, name);
    let ours: Vec<(String, u32)> = layout
        .slots
        .iter()
        .map(|s| (s.name.to_string(), s.offset))
        .collect();
    assert_eq!(
        placed, ours,
        "naga placed `{name}`'s members differently from `ElementLayout`:\n{source}"
    );
    assert_eq!(
        stride, layout.stride,
        "naga's `array<{name}>` stride is not `ElementLayout::stride`:\n{source}"
    );
    assert_eq!(
        span, stride,
        "a struct's span and its array stride must be the same number"
    );
}

fn l1_emitting(emit: &str) -> karakuri_ir::typed::Checked {
    let assignments: String = emit
        .split(", ")
        .map(|attr| match attr {
            "position" | "velocity" | "normal" | "tint" => {
                format!("    {attr} = vec3(0.0, 0.0, 0.0);\n")
            }
            "uv" => "    uv = vec2(0.0, 0.0);\n".to_string(),
            other => format!("    {other} = 0.0;\n"),
        })
        .collect();
    compiled(&format!(
        r#"
proc placed {{
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit {emit}

  element {{
{assignments}  }}
}}
"#
    ))
}

/// **The one case the whole saving comes from: a `vec3` followed by a scalar.**
///
/// `position` is 16-byte aligned and 12 bytes long, so 28..32 is addressable
/// and `size` is placed there rather than at 32 — one 16-byte block for the
/// pair rather than two. That is the packing rule this project relies on, and
/// the assertion is that WGSL agrees it is a rule and not a hope: get it wrong
/// and every element after the first reads four bytes into its predecessor.
#[test]
fn naga_agrees_a_scalar_lands_in_the_padding_a_vec3_leaves() {
    let l1 = l1_emitting("position, size");
    let shader = karakuri_codegen::generate_l1(&l1, &[], &[]);
    validate(&shader.source);
    assert_naga_agrees(&shader.source, "Element", &shader.element_layout);
    // **And the number itself, because agreement is not enough on its own.**
    // The struct's *text* is written from the same slots the offsets are, so a
    // wrong element type — `size` declared as a `vec3` — moves the declaration
    // and the offset together and naga agrees about the wrong thing. These two
    // are what says which packing was agreed on.
    assert_eq!(shader.element_layout.offset_of("size"), 28);
    assert_eq!(shader.element_layout.stride, 32);
}

/// The same check across the shapes a real `emit` list takes: nothing but the
/// two unconditional scalars, a lone vector, two vectors with a scalar closing
/// the second's padding, and a `vec2` — the one alignment between 4 and 16.
#[test]
fn naga_agrees_with_the_element_layout_for_every_emit_shape() {
    for emit in [
        "position",
        "position, velocity, age",
        "position, uv, size",
        "uv, age",
        "position, normal, tint",
    ] {
        let l1 = l1_emitting(emit);
        let shader = karakuri_codegen::generate_l1(&l1, &[], &[]);
        validate(&shader.source);
        assert_naga_agrees(&shader.source, "Element", &shader.element_layout);
    }
}

/// **An L4 addresses the very buffer an L1 wrote**, under a struct it declares
/// itself, so the two have to place every member identically — and this asks a
/// front end rather than comparing the generator to itself.
#[test]
fn naga_agrees_with_the_element_layout_in_a_renderer() {
    let layout = layout_for(&drift_shell());
    let shader = karakuri_codegen::generate_l4(&soft_points(), &layout, &[]);
    validate(&shader.source);
    assert_naga_agrees(&shader.source, "Element", &layout);
}

/// **Both of an L2's structs, which are different shapes in one module.**
///
/// The output carries `copy` where the input does not, so the two disagree
/// about everything after the first eight bytes — and a single struct checked
/// twice would not notice a generator that emitted the input's shape under the
/// output's name.
#[test]
fn naga_agrees_with_both_element_layouts_in_a_deform() {
    let shader = compiled_l2(
        MIRROR,
        &[Attr::Position, Attr::Size],
        karakuri_ir::layout::Synthetic::NONE,
    );
    validate(&shader.source);
    let input = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position, Attr::Size],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );
    assert_naga_agrees(&shader.source, "ElementIn", &input);
    assert_naga_agrees(&shader.source, "ElementOut", &shader.element_layout);
    assert!(
        shader.element_layout.has_slot("copy"),
        "an amplifier's output carries the copy index: {:?}",
        shader.element_layout
    );
}

/// **A derived attribute's stored slot is placed by the same rules**, and it is
/// the one slot no `emit` list mentions: `velocity` exists here because a
/// downstream consumer named it, with the `velocity_lived` flag beside it in
/// the four bytes that `vec3` leaves. A slot nothing declares is exactly where
/// a placement rule is easiest to get wrong unnoticed.
#[test]
fn naga_agrees_about_a_slot_no_procedure_declared() {
    let l1 = l1_emitting("position");
    let shader = karakuri_codegen::generate_l1(&l1, &[Attr::Velocity], &[]);
    validate(&shader.source);
    assert!(shader.element_layout.has_slot("velocity_lived"));
    assert_naga_agrees(&shader.source, "Element", &shader.element_layout);
}

/// **`source` and a Source slot both lower to a uniform read, in every kind
/// that has one**, and the result is WGSL a front end accepts.
///
/// The two halves are one claim about where the value lives. `source` is
/// `u.seed_salt` — the field `Set::prepare` has been writing the geometry's
/// salt into all along, which is why the read needed no new plumbing — and a
/// declared slot is a `u32` of its own beside it, holding whatever geometry an
/// edge named. Neither is per element: nothing lands in a varying, nothing
/// lands in the element struct, and a fullscreen renderer with no element at
/// all reads both.
#[test]
fn source_and_a_source_slot_lower_to_uniform_reads() {
    let layout = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );

    let l1 = compiled(
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 64] = 8

  uses only : Source

  emit position

  element {
    var k = 0.0;
    if source == only { k = 1.0; }
    position = vec3(k, 0.0, 0.0);
  }
}
"#,
    );
    let out = karakuri_codegen::generate_l1(&l1, &[], &[]);
    assert!(
        out.source.contains("u.seed_salt == u.source_only"),
        "`source == only` is two uniform loads: {}",
        out.source
    );
    validate(&out.source);

    let l2 = r#"
proc dissolve {
  kind L2

  uses a : Source
  uses b : Source

  consumes position, size

  mask {
    strength = 0.0;
    if source == a || source == b { strength = 1.0; }
  }

  deform { size = size * 0.2; }
}
"#;
    let out = compiled_l2(
        l2,
        &[Attr::Position, Attr::Size],
        karakuri_ir::layout::Synthetic::NONE,
    );
    // **Two slots, two fields.** One would be an edge answering for both, which
    // is the failure the name on the slot exists to prevent.
    assert!(
        out.source.contains("u.source_a") && out.source.contains("u.source_b"),
        "each slot reads its own uniform field: {}",
        out.source
    );
    validate(&out.source);

    // A per-element renderer, and a fullscreen one — which has no element and
    // reads it all the same, because the value is per chain instance.
    for src in [
        r#"
proc lit {
  kind  L4
  blend additive

  uses only : Source

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    var k = 0.0;
    if source == only { k = 1.0; }
    color = vec4(k, 0.0, 0.0, 1.0);
  }
}
"#,
        r#"
proc march {
  kind  L4
  blend additive

  uses only : Source

  fragment {
    var k = 0.0;
    if source == only { k = 1.0; }
    color = vec4(k, length(ray) * 0.0, 0.0, 1.0);
  }
}
"#,
    ] {
        let l4 = compiled(src);
        let out = karakuri_codegen::generate_l4(&l4, &layout, &[]);
        assert!(
            out.source.contains("u.seed_salt == u.source_only"),
            "an L4 reads both out of its uniform: {}",
            out.source
        );
        assert!(
            !out.source.contains("in.source") && !out.source.contains(".source;"),
            "and neither is a varying or an element field: {}",
            out.source
        );
        validate(&out.source);
    }
}

/// **A procedure that declares no Source slot carries no such uniform field**,
/// which is what keeps this from being a `u32` every module in every Set pays
/// for.
#[test]
fn a_procedure_with_no_source_slot_declares_no_field_for_one() {
    let layout = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );
    let l4 = compiled(
        r#"
proc plain {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment { color = vec4(float(source % 3u) * 0.3, 0.0, 0.0, 1.0); }
}
"#,
    );
    let out = karakuri_codegen::generate_l4(&l4, &layout, &[]);
    assert!(
        !out.source.contains("source_"),
        "no slot, no field: {}",
        out.source
    );
    // And `source` itself still reads, out of the field that was always there.
    assert!(out.source.contains("u.seed_salt"), "{}", out.source);
    assert!(
        out.uniform_layout
            .fields
            .iter()
            .all(|f| !f.name.starts_with("source\u{1}")),
        "the layout carries no slot key either: {:?}",
        out.uniform_layout.fields
    );
    validate(&out.source);
}
