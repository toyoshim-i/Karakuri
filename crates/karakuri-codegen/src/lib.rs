//! WGSL code generation: stage 5 of the validation pipeline described in
//! `docs/ir-spec.md`.
//!
//! `karakuri-ir` gets a `.kir` procedure through parsing, type checking,
//! contract checking, and cost estimation, producing a
//! [`karakuri_ir::typed::Checked`] tree in which every node already carries
//! its resolved type. This crate's only job is to turn that tree into WGSL
//! text plus the binding metadata `karakuri-engine` needs to drive it —
//! nothing here re-derives a type or re-validates a rule the earlier stages
//! already enforce.
//!
//! Two independent lowerings, one per `Kind`:
//!
//! - [`l1::generate_l1`] — `spawn`/`element` as compute entry points. See
//!   the module doc for the state model (prev/next double buffering) and
//!   what is deliberately *not* generated (the compaction scan).
//! - [`l4::generate_l4`] — `vertex`/`fragment` as a render pipeline. See the
//!   module doc for why `topology points` becomes a quad, not a point
//!   primitive.
//!
//! [`layout`] is the contract between this crate's output and the engine
//! that consumes it: group/binding numbers, the uniform struct's field
//! order and byte offsets, and the workgroup size. Treat it as documentation
//! with a compiler behind it, not an implementation detail — see that
//! module's doc comment.

pub mod l1;
pub mod l4;
pub mod layout;
mod lower;
mod prelude;
mod ty;

pub use l1::{generate_l1, L1Shader};
pub use l4::{generate_l4, L4Shader};

use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

/// Dispatches on `checked.kind` and returns whichever of [`L1Shader`] /
/// [`L4Shader`] applies.
#[derive(Debug, Clone)]
pub enum Shader {
    L1(L1Shader),
    L4(L4Shader),
}

/// Lowers a checked procedure to WGSL, picking the L1 or L4 path by
/// `checked.kind`. Prefer [`generate_l1`] / [`generate_l4`] directly when
/// the kind is already known statically.
pub fn generate(checked: &Checked) -> Shader {
    match checked.kind {
        Kind::L1 => Shader::L1(generate_l1(checked)),
        Kind::L4 => Shader::L4(generate_l4(checked)),
    }
}

#[cfg(test)]
mod tests {
    //! End-to-end tests against hand-built `Checked` trees.
    //!
    //! The check pass (`karakuri-ir`'s stages 2-4) does not exist yet, so
    //! these build `Checked` values directly — `typed.rs` is public exactly
    //! so this is possible. That also means nothing here can rely on a
    //! checker having rejected a malformed tree; every fixture is built to
    //! already satisfy the rules (every emitted attribute assigned on every
    //! path, etc.) by hand.
    //!
    //! Two tiers: the first asserts on the emitted text directly, for the
    //! specific lowering rules the brief calls out by name. The second (in
    //! `naga_test.rs`) feeds the output through a real WGSL front end,
    //! because a generator whose output is never compiled will happily keep
    //! emitting plausible nonsense forever.

    use karakuri_ir::builtin::Builtin;
    use karakuri_ir::typed::{Checked, Derivation, TBlock, TExpr, TExprKind, TStmt, Target};
    use karakuri_ir::{Ambient, Attr, BinOp, BlockKind, Kind, Lit, Output, Param, Ty};

    use crate::{generate_l1, generate_l4};

    fn span() -> karakuri_ir::Span {
        karakuri_ir::Span::EMPTY
    }

    fn lit_f(f: f32) -> TExpr {
        TExpr::new(Ty::Float, span(), TExprKind::Lit(Lit::Float(f)))
    }

    fn attr_read(a: Attr) -> TExpr {
        TExpr::new(a.ty(), span(), TExprKind::Attr(a))
    }

    fn empty_checked(name: &str, kind: Kind) -> Checked {
        Checked {
            name: name.to_string(),
            kind,
            topology: None,
            capacity: None,
            blend: None,
            params: Vec::new(),
            emit: Vec::new(),
            consumes: Vec::new(),
            derived: Vec::<Derivation>::new(),
            blocks: Vec::new(),
            cost: None,
            span: span(),
        }
    }

    /// A minimal L1 procedure: emits `position` (`vec3`), and `element`
    /// first assigns `position`, then immediately reads it back into a
    /// `let`. This is the "read of an attribute after an assignment to that
    /// same attribute still reads prev" test: if the generator ever cached
    /// an attribute write in a local and reused it for the following read,
    /// this fixture is built to catch it, because the second `let`'s value
    /// would then differ textually from a `prev_position` reference.
    fn read_after_write_proc() -> Checked {
        let mut p = empty_checked("read_after_write", Kind::L1);
        p.emit = vec![Attr::Position];

        let assign_position = TStmt::Assign {
            target: Target::Attr(Attr::Position),
            value: TExpr::new(
                Ty::Vec3,
                span(),
                TExprKind::Construct { args: vec![lit_f(1.0), lit_f(2.0), lit_f(3.0)] },
            ),
            span: span(),
        };
        // Reads `position` again *after* writing it above. Must still read
        // the previous frame's buffer, not the value just assigned.
        let reread = TStmt::Let { name: "again".to_string(), value: attr_read(Attr::Position), span: span() };

        p.blocks.push(TBlock { kind: BlockKind::Element, stmts: vec![assign_position, reread], span: span() });
        p
    }

    #[test]
    fn attribute_read_after_assignment_still_reads_prev_buffer() {
        let shader = generate_l1(&read_after_write_proc());
        // The write goes to `next_position`; the following read must still
        // be `prev_position`, never a reference to a local that captured
        // the write.
        assert!(shader.source.contains("next_position[i] ="), "{}", shader.source);
        assert!(
            shader.source.contains("let usr_again = prev_position[i].xyz;"),
            "expected the re-read to reference prev_position, got:\n{}",
            shader.source
        );
    }

    /// A block calling `fbm(position, 3)` — the octave count must be
    /// unrolled into three `perlin` terms at generation time, with no
    /// runtime loop and no call to a function literally named `fbm` (WGSL
    /// has no preprocessor to unroll one for us, so this crate must not
    /// emit one).
    fn fbm_proc() -> Checked {
        let mut p = empty_checked("fbm_user", Kind::L1);
        p.emit = vec![Attr::Position, Attr::Age];
        let call = TExpr::new(
            Ty::Float,
            span(),
            TExprKind::Builtin {
                func: Builtin::Fbm,
                args: vec![
                    attr_read(Attr::Position),
                    TExpr::new(Ty::Int, span(), TExprKind::Lit(Lit::Int(3))),
                ],
            },
        );
        let assign = TStmt::Assign { target: Target::Attr(Attr::Age), value: call, span: span() };
        p.blocks.push(TBlock { kind: BlockKind::Element, stmts: vec![assign], span: span() });
        p
    }

    #[test]
    fn fbm_is_unrolled_at_generation_time() {
        let shader = generate_l1(&fbm_proc());
        // Exactly one `perlin` helper definition, plus exactly three call
        // sites from unrolling `fbm(position, 3)` — counting bare
        // `"perlin("` would also match the helper's own `fn perlin(`.
        assert_eq!(shader.source.matches("fn perlin(").count(), 1, "{}", shader.source);
        assert_eq!(
            shader.source.matches("perlin(prev_position").count(),
            3,
            "fbm(_, 3) should unroll to exactly three perlin() call sites:\n{}",
            shader.source
        );
        assert!(!shader.source.contains("fbm("), "no call to fbm() should survive lowering:\n{}", shader.source);
        assert!(!shader.source.contains("for "), "an unrolled fbm should need no runtime loop:\n{}", shader.source);
    }

    /// `%` on a float attribute must route through the `mod_f32` helper
    /// (IR `mod` semantics, always non-negative) — and that helper must be
    /// entirely absent when nothing in the procedure uses `%` on a float.
    fn float_rem_proc() -> Checked {
        let mut p = empty_checked("wrap_age", Kind::L1);
        p.emit = vec![Attr::Age];
        let rem = TExpr::new(
            Ty::Float,
            span(),
            TExprKind::Binary { op: BinOp::Rem, lhs: Box::new(attr_read(Attr::Age)), rhs: Box::new(lit_f(1.0)) },
        );
        let assign = TStmt::Assign { target: Target::Attr(Attr::Age), value: rem, span: span() };
        p.blocks.push(TBlock { kind: BlockKind::Element, stmts: vec![assign], span: span() });
        p
    }

    #[test]
    fn mod_helper_appears_only_when_percent_is_used_on_a_float() {
        let with_rem = generate_l1(&float_rem_proc());
        assert!(with_rem.source.contains("fn mod_f32("), "{}", with_rem.source);
        assert!(with_rem.source.contains("mod_f32(prev_age[i].x, 1.0)"), "{}", with_rem.source);

        // A procedure that never uses `%` on a float must not carry the
        // helper at all.
        let mut p = empty_checked("no_mod", Kind::L1);
        p.emit = vec![Attr::Age];
        let assign = TStmt::Assign { target: Target::Attr(Attr::Age), value: lit_f(1.0), span: span() };
        p.blocks.push(TBlock { kind: BlockKind::Element, stmts: vec![assign], span: span() });
        let without_rem = generate_l1(&p);
        assert!(!without_rem.source.contains("mod_f32"), "{}", without_rem.source);
    }

    #[test]
    fn integer_percent_uses_the_native_operator_not_the_helper() {
        let mut p = empty_checked("bucket", Kind::L1);
        p.emit = vec![];
        // `seed % 512u` — the ir-spec's own example of structured layout
        // from an ordinal. `int`/`uint` `%` is ordinary remainder, native
        // in WGSL, and must not pull in a mod_* helper.
        let rem = TExpr::new(
            Ty::Uint,
            span(),
            TExprKind::Binary {
                op: BinOp::Rem,
                lhs: Box::new(TExpr::new(Ty::Uint, span(), TExprKind::Ambient(Ambient::Seed))),
                rhs: Box::new(TExpr::new(Ty::Uint, span(), TExprKind::Lit(Lit::Uint(512)))),
            },
        );
        let let_stmt = TStmt::Let { name: "bucket".to_string(), value: rem, span: span() };
        let assign = TStmt::Assign {
            target: Target::Attr(Attr::Age),
            value: lit_f(0.0),
            span: span(),
        };
        p.emit = vec![Attr::Age];
        p.blocks.push(TBlock { kind: BlockKind::Element, stmts: vec![let_stmt, assign], span: span() });
        let shader = generate_l1(&p);
        assert!(shader.source.contains("let usr_bucket = (seed % 512u);"), "{}", shader.source);
        assert!(!shader.source.contains("mod_"), "{}", shader.source);
    }

    #[test]
    fn uniform_struct_total_size_is_sixteen_byte_aligned() {
        let mut p = empty_checked("params", Kind::L1);
        p.params = vec![
            Param { name: "radius".to_string(), ty: Ty::Float, min: 0.0, max: 1.0, default: dummy_expr(), span: span() },
            Param { name: "glow".to_string(), ty: Ty::Vec3, min: 0.0, max: 1.0, default: dummy_expr(), span: span() },
        ];
        p.emit = vec![Attr::Age];
        let assign = TStmt::Assign { target: Target::Attr(Attr::Age), value: lit_f(0.0), span: span() };
        p.blocks.push(TBlock { kind: BlockKind::Element, stmts: vec![assign], span: span() });

        let shader = generate_l1(&p);
        assert_eq!(shader.uniform_layout.total_size % 16, 0, "{:#?}", shader.uniform_layout);
        // The padded size must be large enough to hold every field, not
        // merely a multiple of 16 by accident.
        let last_end = shader.uniform_layout.fields.iter().map(|f| f.offset + f.size).max().unwrap_or(0);
        assert!(shader.uniform_layout.total_size >= last_end);
        assert!(shader.source.contains("_pad"), "expected an explicit trailing pad field:\n{}", shader.source);
    }

    fn dummy_expr() -> karakuri_ir::Expr {
        karakuri_ir::Expr::Lit { value: Lit::Float(0.0), span: span() }
    }

    /// An L4 fixture matching the ir-spec's `soft_points` shape closely
    /// enough to exercise varying selection, `hsv_to_rgb`, and quad
    /// expansion together: `vertex` writes `clip`/`point_size` from
    /// `position`/`camera`; `fragment` reads `point_coord` and `seed`.
    fn l4_proc() -> Checked {
        let mut p = empty_checked("soft_points", Kind::L4);
        p.consumes = vec![Attr::Position];
        p.blend = Some(karakuri_ir::Blend::Additive);

        let clip = TStmt::Assign {
            target: Target::Output(Output::Clip),
            value: TExpr::new(
                Ty::Vec4,
                span(),
                TExprKind::Binary {
                    op: BinOp::Mul,
                    lhs: Box::new(TExpr::new(Ty::Mat4, span(), TExprKind::Ambient(Ambient::Camera))),
                    rhs: Box::new(TExpr::new(
                        Ty::Vec4,
                        span(),
                        TExprKind::Construct { args: vec![attr_read(Attr::Position), lit_f(1.0)] },
                    )),
                },
            ),
            span: span(),
        };
        let point_size =
            TStmt::Assign { target: Target::Output(Output::PointSize), value: lit_f(6.0), span: span() };
        let vertex = TBlock { kind: BlockKind::Vertex, stmts: vec![clip, point_size], span: span() };

        let hsv = TExpr::new(
            Ty::Vec3,
            span(),
            TExprKind::Builtin {
                func: Builtin::HsvToRgb,
                args: vec![TExpr::new(
                    Ty::Vec3,
                    span(),
                    TExprKind::Construct { args: vec![lit_f(0.5), lit_f(0.7), lit_f(1.0)] },
                )],
            },
        );
        let color_rgb = TStmt::Let { name: "c".to_string(), value: hsv, span: span() };
        let alpha = TExpr::new(
            Ty::Float,
            span(),
            TExprKind::Builtin {
                func: Builtin::Length,
                args: vec![TExpr::new(Ty::Vec2, span(), TExprKind::Ambient(Ambient::PointCoord))],
            },
        );
        let alpha_let = TStmt::Let { name: "a".to_string(), value: alpha, span: span() };
        let color = TStmt::Assign {
            target: Target::Output(Output::Color),
            value: TExpr::new(
                Ty::Vec4,
                span(),
                TExprKind::Construct {
                    args: vec![
                        TExpr::new(Ty::Vec3, span(), TExprKind::Local("c".to_string())),
                        TExpr::new(Ty::Float, span(), TExprKind::Local("a".to_string())),
                    ],
                },
            ),
            span: span(),
        };
        let fragment =
            TBlock { kind: BlockKind::Fragment, stmts: vec![color_rgb, alpha_let, color], span: span() };

        p.blocks = vec![vertex, fragment];
        p
    }

    #[test]
    fn l4_quad_expansion_and_hsv_to_rgb_wiring() {
        let shader = generate_l4(&l4_proc());
        let src = &shader.source;
        assert!(src.contains("@builtin(vertex_index) corner_idx: u32"), "{src}");
        assert!(src.contains("@builtin(instance_index) elem: u32"), "{src}");
        assert!(src.contains("fn hsv_to_rgb("), "{src}");
        // hsv_to_rgb must end by converting sRGB->linear, not linear->sRGB —
        // getting this backwards is the trap the brief calls out by name.
        assert!(src.contains("return srgb_to_linear(srgb);"), "{src}");
        assert!(!src.contains("return linear_to_srgb(srgb);"), "{src}");
        // position is consumed but never read in fragment, so it must not
        // become a varying.
        assert!(!src.contains("position: vec3<f32>,\n"), "{src}");
        // point_coord is fragment-only ambient and IS used in fragment.
        assert!(src.contains("in.point_coord"), "{src}");
    }
}
