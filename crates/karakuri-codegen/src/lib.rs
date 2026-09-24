//! WGSL code generation for checked `.kir` procedure ASTs.
//!
//! Generates WGSL shaders and binding metadata from [`karakuri_ir::typed::Checked`]:
//! - [`l1::generate_l1`]: Compute pipelines for element spawn and update.
//! - [`l2::generate_l2`]: Deformation passes applied over element streams.
//! - [`l3::generate_l3`]: Compute pass writing camera uniform matrices.
//! - [`l4::generate_l4`]: Render pipelines for element rendering.
//! - [`l5::generate_l5`]: Post-processing / master chain effects.
//! - [`field::generate_field`]: Spliced WGSL functions for distance fields.
//! - [`layout`]: Buffer layout, uniform alignment, and binding slot definitions.

pub mod field;

/// Returns the declared Field slots evaluated by `checked`, ordered by declaration.
///
/// Uses the cost estimate to filter out declared slots that are never evaluated,
/// ensuring that unused field procedures do not introduce unused parameters or shader dependencies.
pub(crate) fn evaluated_slots(checked: &karakuri_ir::typed::Checked) -> Vec<&str> {
    let declared = checked.field_slots();
    if declared.is_empty() {
        return Vec::new();
    }
    let Ok(cost) = karakuri_ir::cost::estimate(checked) else {
        return Vec::new();
    };
    declared
        .into_iter()
        .filter(|s| cost.field_calls.slot(s).is_some_and(|c| c.total > 0))
        .collect()
}

/// Binding of declared caller Field slot names to their bound Field procedures.
///
/// Resolved externally by the Set before shader generation.
pub type Bound<'a> = &'a [(&'a str, &'a Checked)];

/// Returns the field shader splices required for the given checked procedure.
pub(crate) fn splices(
    checked: &karakuri_ir::typed::Checked,
    fields: Bound<'_>,
) -> Vec<field::FieldShader> {
    evaluated_slots(checked)
        .into_iter()
        .filter_map(|slot| {
            let (_, f) = fields.iter().find(|(name, _)| *name == slot)?;
            Some(field::generate_field(f, slot))
        })
        .collect()
}
pub mod ast;
pub mod fusion;
pub mod l1;
pub mod l2;
pub mod l3;
pub mod l4;
pub mod l5;
pub mod layout;
mod lower;
mod prelude;
mod ty;

pub use fusion::{fuse_l2_into_l4, FusedShader};
pub use l1::{generate_l1, L1Shader};
pub use l2::{generate_l2, L2Shader};
pub use l3::{generate_l3, L3Shader};
pub use l4::{generate_l4, L4Shader};
pub use l5::{generate_l5, L5Shader};

use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

use karakuri_ir::layout::ElementLayout;

/// Dispatches on `checked.kind` and returns whichever of [`L1Shader`] /
/// [`L4Shader`] applies.
#[derive(Debug, Clone)]
pub enum Shader {
    L1(L1Shader),
    L3(L3Shader),
    L4(L4Shader),
    L5(L5Shader),
}

/// Lowers a checked procedure to WGSL, selecting the pipeline generator by `checked.kind`.
///
/// `elements` provides the paired L1 procedure's [`ElementLayout`], which is required
/// when lowering an L4 procedure.
pub fn generate(checked: &Checked, elements: Option<&ElementLayout>) -> Shader {
    match checked.kind {
        Kind::L1 => Shader::L1(generate_l1(checked, &[], &[])),
        // L2 procedures require chain context and must be generated via `generate_l2`.
        Kind::L2 => panic!("an L2 is generated against its position in a chain: call generate_l2"),
        // Field procedures are spliced into callers and must be generated via `generate_field`.
        Kind::Field => panic!("a field lowers into its callers: call generate_field"),
        Kind::L3 => Shader::L3(generate_l3(checked, &[])),
        Kind::L5 => Shader::L5(generate_l5(checked)),
        Kind::L4 => {
            let elements = elements.expect("an L4 procedure needs its paired L1's ElementLayout");
            Shader::L4(generate_l4(checked, elements, &[]))
        }
    }
}

#[cfg(test)]
mod tests {
    //! Tests verifying code generation against hand-constructed `Checked` AST fixtures.

    use karakuri_ir::builtin::Builtin;
    use karakuri_ir::typed::{Checked, TBlock, TExpr, TExprKind, TStmt, Target};
    use karakuri_ir::{Ambient, Attr, BinOp, BlockKind, Kind, Lit, Output, Param, Topology, Ty};

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
            // Not `None`: an L4's topology is inferred by the check pass, and
            // `generate_l4` reads it to choose the quad expansion. These are
            // hand-built stand-ins for checked trees, so they carry what
            // `check` would have put here.
            topology: Some(Topology::Points),
            capacity: None,
            amplify: None,
            uses: Vec::new(),
            retains: false,
            blend: None,
            params: Vec::new(),
            emit: Vec::new(),
            consumes: Vec::new(),
            blocks: Vec::new(),
            cost: None,
            // Hand-built fixtures: `check` is what decides this, and these never
            // run it. `false` is the conservative side and nothing here reads it.
            closed_form: false,
            reads_beats: false,
            span: span(),
        }
    }

    /// Fixture verifying that an attribute read after an assignment in the same step
    /// continues to reference the previous element state (`prev`).
    fn read_after_write_proc() -> Checked {
        let mut p = empty_checked("read_after_write", Kind::L1);
        p.emit = vec![Attr::Position];

        let assign_position = TStmt::Assign {
            target: Target::Attr(Attr::Position),
            value: TExpr::new(
                Ty::Vec3,
                span(),
                TExprKind::Construct {
                    args: vec![lit_f(1.0), lit_f(2.0), lit_f(3.0)],
                },
            ),
            span: span(),
        };
        // Reads `position` again *after* writing it above. Must still read
        // the previous frame's buffer, not the value just assigned.
        let reread = TStmt::Let {
            name: "again".to_string(),
            value: attr_read(Attr::Position),
            span: span(),
        };

        p.blocks.push(TBlock {
            kind: BlockKind::Element,
            stmts: vec![assign_position, reread],
            span: span(),
        });
        p
    }

    #[test]
    fn attribute_read_after_assignment_still_reads_prev_buffer() {
        let shader = generate_l1(&read_after_write_proc(), &[], &[]);
        // The write goes to `next[out]`; the following read must still be
        // `prev[i]`, never a reference to a local that captured the write.
        // Read and write index are separate expressions precisely because
        // compaction makes them different slots.
        assert!(
            shader.source.contains("next[out].position ="),
            "{}",
            shader.source
        );
        assert!(
            shader.source.contains("let usr_again = prev[i].position;"),
            "expected the re-read to reference prev[i].position, got:\n{}",
            shader.source
        );
    }

    /// A procedure with neither `spawn` nor `kill()` cannot change its live
    /// set, so the engine skips the scan for it — and `element` must then
    /// not read the `dest` buffer that scan would have filled, nor declare
    /// a binding for it.
    #[test]
    fn a_static_procedure_neither_binds_nor_reads_the_destination_indices() {
        let shader = generate_l1(&read_after_write_proc(), &[], &[]);
        assert!(
            !shader.compacted,
            "no spawn block and no kill() is a static procedure"
        );
        assert!(
            !shader.source.contains("dest"),
            "static `element` must not touch dest:\n{}",
            shader.source
        );
        assert!(
            !shader.source.contains("prev_alive[i]"),
            "static `element` has no dead elements to skip:\n{}",
            shader.source
        );
        assert!(shader.source.contains("let out = i;"), "{}", shader.source);
    }

    /// The same procedure with one `kill()` added, nested inside an `if` so
    /// this also covers the recursive walk: it becomes compacted, and
    /// `element` gains both the destination read and the skip.
    #[test]
    fn a_kill_anywhere_in_the_element_block_makes_a_procedure_compacted() {
        let mut p = read_after_write_proc();
        let element = p
            .blocks
            .last_mut()
            .expect("the fixture has an element block");
        element.stmts.push(TStmt::If {
            cond: TExpr::new(Ty::Bool, span(), TExprKind::Lit(Lit::Bool(true))),
            then: vec![TStmt::For {
                var: "n".to_string(),
                start: 0,
                end: 2,
                body: vec![TStmt::Kill { span: span() }],
                span: span(),
            }],
            els: vec![],
            span: span(),
        });

        let shader = generate_l1(&p, &[], &[]);
        assert!(
            shader.compacted,
            "a kill() inside an if inside a for still kills"
        );
        assert!(!shader.has_spawn, "no spawn block was added");
        assert!(
            shader.source.contains("let out = dest[i];"),
            "{}",
            shader.source
        );
        assert!(
            shader.source.contains("if prev_alive[i] == 0u { return; }"),
            "{}",
            shader.source
        );
        assert!(
            shader.source.contains("next[out].position ="),
            "{}",
            shader.source
        );
    }

    /// Returns a fixture procedure invoking `fbm(position, 3)`.
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
        let assign = TStmt::Assign {
            target: Target::Attr(Attr::Age),
            value: call,
            span: span(),
        };
        p.blocks.push(TBlock {
            kind: BlockKind::Element,
            stmts: vec![assign],
            span: span(),
        });
        p
    }

    #[test]
    fn fbm_is_unrolled_at_generation_time() {
        let shader = generate_l1(&fbm_proc(), &[], &[]);
        // Exactly one `perlin` helper definition, plus exactly three call
        // sites from unrolling `fbm(position, 3)` — counting bare
        // `"perlin("` would also match the helper's own `fn perlin(`.
        assert_eq!(
            shader.source.matches("fn perlin(").count(),
            1,
            "{}",
            shader.source
        );
        assert_eq!(
            shader.source.matches("perlin(prev[i].position").count(),
            3,
            "fbm(_, 3) should unroll to exactly three perlin() call sites:\n{}",
            shader.source
        );
        assert!(
            !shader.source.contains("fbm("),
            "no call to fbm() should survive lowering:\n{}",
            shader.source
        );
        assert!(
            !shader.source.contains("for "),
            "an unrolled fbm should need no runtime loop:\n{}",
            shader.source
        );
    }

    /// `%` on a float attribute must route through the `mod_f32` helper
    /// (IR `mod` semantics, which take the sign of the **divisor** rather than
    /// being always non-negative) — and that helper must be
    /// entirely absent when nothing in the procedure uses `%` on a float.
    fn float_rem_proc() -> Checked {
        let mut p = empty_checked("wrap_age", Kind::L1);
        p.emit = vec![Attr::Age];
        let rem = TExpr::new(
            Ty::Float,
            span(),
            TExprKind::Binary {
                op: BinOp::Rem,
                lhs: Box::new(attr_read(Attr::Age)),
                rhs: Box::new(lit_f(1.0)),
            },
        );
        let assign = TStmt::Assign {
            target: Target::Attr(Attr::Age),
            value: rem,
            span: span(),
        };
        p.blocks.push(TBlock {
            kind: BlockKind::Element,
            stmts: vec![assign],
            span: span(),
        });
        p
    }

    #[test]
    fn mod_helper_appears_only_when_percent_is_used_on_a_float() {
        let with_rem = generate_l1(&float_rem_proc(), &[], &[]);
        assert!(
            with_rem.source.contains("fn mod_f32("),
            "{}",
            with_rem.source
        );
        assert!(
            with_rem.source.contains("mod_f32(prev[i].age, 1.0)"),
            "{}",
            with_rem.source
        );

        // A procedure that never uses `%` on a float must not carry the
        // helper at all.
        let mut p = empty_checked("no_mod", Kind::L1);
        p.emit = vec![Attr::Age];
        let assign = TStmt::Assign {
            target: Target::Attr(Attr::Age),
            value: lit_f(1.0),
            span: span(),
        };
        p.blocks.push(TBlock {
            kind: BlockKind::Element,
            stmts: vec![assign],
            span: span(),
        });
        let without_rem = generate_l1(&p, &[], &[]);
        assert!(
            !without_rem.source.contains("mod_f32"),
            "{}",
            without_rem.source
        );
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
                lhs: Box::new(TExpr::new(
                    Ty::Uint,
                    span(),
                    TExprKind::Ambient(Ambient::Seed),
                )),
                rhs: Box::new(TExpr::new(Ty::Uint, span(), TExprKind::Lit(Lit::Uint(512)))),
            },
        );
        let let_stmt = TStmt::Let {
            name: "bucket".to_string(),
            value: rem,
            span: span(),
        };
        let assign = TStmt::Assign {
            target: Target::Attr(Attr::Age),
            value: lit_f(0.0),
            span: span(),
        };
        p.emit = vec![Attr::Age];
        p.blocks.push(TBlock {
            kind: BlockKind::Element,
            stmts: vec![let_stmt, assign],
            span: span(),
        });
        let shader = generate_l1(&p, &[], &[]);
        assert!(
            shader.source.contains("let usr_bucket = (seed % 512u);"),
            "{}",
            shader.source
        );
        assert!(!shader.source.contains("mod_"), "{}", shader.source);
    }

    #[test]
    fn uniform_struct_total_size_is_sixteen_byte_aligned() {
        let mut p = empty_checked("params", Kind::L1);
        p.params = vec![
            Param {
                name: "radius".to_string(),
                ty: Ty::Float,
                min: 0.0,
                max: 1.0,
                default: dummy_expr(),
                span: span(),
            },
            Param {
                name: "glow".to_string(),
                ty: Ty::Vec3,
                min: 0.0,
                max: 1.0,
                default: dummy_expr(),
                span: span(),
            },
        ];
        p.emit = vec![Attr::Age];
        let assign = TStmt::Assign {
            target: Target::Attr(Attr::Age),
            value: lit_f(0.0),
            span: span(),
        };
        p.blocks.push(TBlock {
            kind: BlockKind::Element,
            stmts: vec![assign],
            span: span(),
        });

        let shader = generate_l1(&p, &[], &[]);
        assert_eq!(
            shader.uniform_layout.total_size % 16,
            0,
            "{:#?}",
            shader.uniform_layout
        );
        // The padded size must be large enough to hold every field, not
        // merely a multiple of 16 by accident.
        let last_end = shader
            .uniform_layout
            .fields
            .iter()
            .map(|f| f.offset + f.size)
            .max()
            .unwrap_or(0);
        assert!(shader.uniform_layout.total_size >= last_end);
        assert!(
            shader.source.contains("_pad"),
            "expected an explicit trailing pad field:\n{}",
            shader.source
        );
    }

    fn dummy_expr() -> karakuri_ir::Expr {
        karakuri_ir::Expr::Lit {
            value: Lit::Float(0.0),
            span: span(),
        }
    }

    /// An L4 fixture matching the ir-spec's `soft_points` shape closely
    /// enough to exercise varying selection, `hsv_to_rgb`, and quad
    /// expansion together: `vertex` writes `clip`/`point_rate` from
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
                    lhs: Box::new(TExpr::new(
                        Ty::Mat4,
                        span(),
                        TExprKind::Ambient(Ambient::Camera),
                    )),
                    rhs: Box::new(TExpr::new(
                        Ty::Vec4,
                        span(),
                        TExprKind::Construct {
                            args: vec![attr_read(Attr::Position), lit_f(1.0)],
                        },
                    )),
                },
            ),
            span: span(),
        };
        let point_rate = TStmt::Assign {
            target: Target::Output(Output::PointRate),
            value: lit_f(0.008),
            span: span(),
        };
        let vertex = TBlock {
            kind: BlockKind::Vertex,
            stmts: vec![clip, point_rate],
            span: span(),
        };

        let hsv = TExpr::new(
            Ty::Vec3,
            span(),
            TExprKind::Builtin {
                func: Builtin::HsvToRgb,
                args: vec![TExpr::new(
                    Ty::Vec3,
                    span(),
                    TExprKind::Construct {
                        args: vec![lit_f(0.5), lit_f(0.7), lit_f(1.0)],
                    },
                )],
            },
        );
        let color_rgb = TStmt::Let {
            name: "c".to_string(),
            value: hsv,
            span: span(),
        };
        let alpha = TExpr::new(
            Ty::Float,
            span(),
            TExprKind::Builtin {
                func: Builtin::Length,
                args: vec![TExpr::new(
                    Ty::Vec2,
                    span(),
                    TExprKind::Ambient(Ambient::PointCoord),
                )],
            },
        );
        let alpha_let = TStmt::Let {
            name: "a".to_string(),
            value: alpha,
            span: span(),
        };
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
        let fragment = TBlock {
            kind: BlockKind::Fragment,
            stmts: vec![color_rgb, alpha_let, color],
            span: span(),
        };

        p.blocks = vec![vertex, fragment];
        p
    }

    #[test]
    fn l4_quad_expansion_and_hsv_to_rgb_wiring() {
        let elements = karakuri_ir::layout::generate_element_layout(
            &[Attr::Position],
            karakuri_ir::layout::Synthetic::NONE,
            &[],
        );
        let shader = generate_l4(&l4_proc(), &elements, &[]);
        let src = &shader.source;
        assert!(
            src.contains("@builtin(vertex_index) corner_idx: u32"),
            "{src}"
        );
        assert!(src.contains("@builtin(instance_index) elem: u32"), "{src}");
        assert!(src.contains("fn hsv_to_rgb("), "{src}");
        // hsv_to_rgb must end by converting sRGB->linear, not linear->sRGB —
        // getting this backwards is the trap the brief calls out by name.
        assert!(src.contains("return srgb_to_linear(srgb);"), "{src}");
        assert!(!src.contains("return linear_to_srgb(srgb);"), "{src}");
        // `position` is consumed but not read in fragment, so it must not become a varying.
        assert!(!src.contains(") position: vec3<f32>,"), "{src}");
        // point_coord is fragment-only ambient and IS used in fragment.
        assert!(src.contains("in.point_coord"), "{src}");
    }
}
