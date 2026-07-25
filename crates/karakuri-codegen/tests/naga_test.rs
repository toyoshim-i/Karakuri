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
use karakuri_ir::typed::{Checked, Derivation, TBlock, TExpr, TExprKind, TStmt, Target};
use karakuri_ir::{Ambient, Attr, BinOp, Blend, BlockKind, Kind, Lit, Output, Param, Span, Topology, Ty};

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
    TExpr::new(ty, span(), TExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) })
}

fn call(func: Builtin, args: Vec<TExpr>, ty: Ty) -> TExpr {
    TExpr::new(ty, span(), TExprKind::Builtin { func, args })
}

fn construct(ty: Ty, args: Vec<TExpr>) -> TExpr {
    TExpr::new(ty, span(), TExprKind::Construct { args })
}

fn let_(name: &str, value: TExpr) -> TStmt {
    TStmt::Let { name: name.to_string(), value, span: span() }
}

fn assign_attr(a: Attr, value: TExpr) -> TStmt {
    TStmt::Assign { target: Target::Attr(a), value, span: span() }
}

fn assign_output(o: Output, value: TExpr) -> TStmt {
    TStmt::Assign { target: Target::Output(o), value, span: span() }
}

fn param_decl(name: &str, ty: Ty, min: f32, max: f32) -> Param {
    Param { name: name.to_string(), ty, min, max, default: dummy_default(), span: span() }
}

fn dummy_default() -> karakuri_ir::Expr {
    karakuri_ir::Expr::Lit { value: Lit::Float(0.0), span: span() }
}

/// Shaped like the ir-spec's `drift_shell` L1 example.
fn drift_shell() -> Checked {
    let spawn = TBlock {
        kind: BlockKind::Spawn,
        span: span(),
        stmts: vec![
            let_("uu", call(Builtin::Hash1, vec![ambient(Ambient::Seed, Ty::Uint)], Ty::Float)),
            let_(
                "vv",
                call(
                    Builtin::Hash1,
                    vec![bin(BinOp::Add, ambient(Ambient::Seed, Ty::Uint), lit_u(1000), Ty::Uint)],
                    Ty::Float,
                ),
            ),
            assign_attr(
                Attr::Position,
                bin(
                    BinOp::Mul,
                    call(Builtin::SpherePoint, vec![local("uu", Ty::Float), local("vv", Ty::Float)], Ty::Vec3),
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
                vec![lit_f(0.0), bin(BinOp::Mul, ambient(Ambient::T, Ty::Float), lit_f(0.1), Ty::Float), lit_f(0.0)],
            ),
            Ty::Vec3,
        )],
        Ty::Vec3,
    );
    let flow = bin(BinOp::Mul, flow, param("turbulence", Ty::Float), Ty::Vec3);
    let new_v = bin(
        BinOp::Add,
        bin(BinOp::Mul, attr(Attr::Velocity), lit_f(0.96), Ty::Vec3),
        bin(BinOp::Mul, local("flow", Ty::Vec3), ambient(Ambient::Dt, Ty::Float), Ty::Vec3),
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
                    bin(BinOp::Mul, local("v", Ty::Vec3), ambient(Ambient::Dt, Ty::Float), Ty::Vec3),
                    Ty::Vec3,
                ),
            ),
            assign_attr(
                Attr::Age,
                bin(BinOp::Add, attr(Attr::Age), ambient(Ambient::Dt, Ty::Float), Ty::Float),
            ),
            TStmt::If {
                cond: bin(BinOp::Gt, attr(Attr::Age), param("lifetime", Ty::Float), Ty::Bool),
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
        blend: None,
        params: vec![
            param_decl("spawn_rate", Ty::Float, 0.0, 40000.0),
            param_decl("radius", Ty::Float, 0.1, 8.0),
            param_decl("turbulence", Ty::Float, 0.0, 3.0),
            param_decl("lifetime", Ty::Float, 0.5, 20.0),
        ],
        emit: vec![Attr::Position, Attr::Velocity, Attr::Age],
        consumes: vec![],
        derived: Vec::<Derivation>::new(),
        blocks: vec![spawn, element],
        cost: None,
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
    let clamped = call(Builtin::Clamp, vec![speed_term, lit_f(0.0), lit_f(1.0)], Ty::Float);
    let point_size = bin(
        BinOp::Mul,
        param("point_scale", Ty::Float),
        bin(BinOp::Add, lit_f(0.3), bin(BinOp::Mul, lit_f(0.7), clamped, Ty::Float), Ty::Float),
        Ty::Float,
    );
    let vertex = TBlock {
        kind: BlockKind::Vertex,
        span: span(),
        stmts: vec![assign_output(Output::Clip, clip), assign_output(Output::PointSize, point_size)],
    };

    let d = call(
        Builtin::Length,
        vec![bin(
            BinOp::Sub,
            bin(BinOp::Mul, ambient(Ambient::PointCoord, Ty::Vec2), lit_f(2.0), Ty::Vec2),
            construct(Ty::Vec2, vec![lit_f(1.0)]),
            Ty::Vec2,
        )],
        Ty::Float,
    );
    let a = call(
        Builtin::Pow,
        vec![
            call(Builtin::Max, vec![lit_f(0.0), bin(BinOp::Sub, lit_f(1.0), local("d", Ty::Float), Ty::Float)], Ty::Float),
            param("falloff", Ty::Float),
        ],
        Ty::Float,
    );
    let hue_jitter = bin(
        BinOp::Add,
        param("hue", Ty::Float),
        bin(BinOp::Mul, call(Builtin::Hash1, vec![ambient(Ambient::Seed, Ty::Uint)], Ty::Float), lit_f(0.05), Ty::Float),
        Ty::Float,
    );
    let c = call(Builtin::HsvToRgb, vec![construct(Ty::Vec3, vec![hue_jitter, lit_f(0.7), lit_f(1.0)])], Ty::Vec3);
    let color = construct(
        Ty::Vec4,
        vec![bin(BinOp::Mul, local("c", Ty::Vec3), param("exposure", Ty::Float), Ty::Vec3), local("a", Ty::Float)],
    );
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: vec![let_("d", d), let_("a", a), let_("c", c), assign_output(Output::Color, color)],
    };

    Checked {
        name: "soft_points".to_string(),
        kind: Kind::L4,
        topology: None,
        capacity: None,
        blend: Some(Blend::Additive),
        params: vec![
            param_decl("point_scale", Ty::Float, 0.5, 40.0),
            param_decl("hue", Ty::Float, 0.0, 1.0),
            param_decl("exposure", Ty::Float, 0.0, 8.0),
            param_decl("falloff", Ty::Float, 0.5, 8.0),
        ],
        emit: vec![],
        consumes: vec![Attr::Position, Attr::Velocity, Attr::Age],
        derived: Vec::<Derivation>::new(),
        blocks: vec![vertex, fragment],
        cost: None,
        span: span(),
    }
}

fn validate(source: &str) {
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|e| {
        panic!("WGSL failed to parse:\n{}\n\n---- source ----\n{source}", e.emit_to_string(source))
    });
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|e| panic!("WGSL failed validation: {e}\n\n---- source ----\n{source}"));
}

#[test]
fn drift_shell_l1_compiles_and_validates() {
    let shader = karakuri_codegen::generate_l1(&drift_shell());
    validate(&shader.source);
}

#[test]
fn soft_points_l4_compiles_and_validates() {
    let shader = karakuri_codegen::generate_l4(&soft_points());
    validate(&shader.source);
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
    let module = naga::front::wgsl::parse_str(broken).expect("this fixture is syntactically valid WGSL");
    let result = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module);
    assert!(result.is_err(), "expected a return-type mismatch to fail validation");
}
