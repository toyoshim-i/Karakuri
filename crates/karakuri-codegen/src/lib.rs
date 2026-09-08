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
//! One independent lowering per `Kind`:
//!
//! - [`l1::generate_l1`] — `spawn`/`element` as compute entry points. See
//!   the module doc for the state model (prev/next double buffering) and
//!   what is deliberately *not* generated (the compaction scan).
//! - [`l3::generate_l3`] — `camera` as one compute invocation writing the
//!   camera state. See the module doc for why a camera on the clock alone is
//!   lowered to a GPU pass anyway.
//! - [`l4::generate_l4`] — `vertex`/`fragment` as a render pipeline. See the
//!   module doc for why `topology points` becomes a quad, not a point
//!   primitive.
//!
//! [`layout`] is the contract between this crate's output and the engine
//! that consumes it: group/binding numbers, the uniform struct's field
//! order and byte offsets, and the workgroup size. Treat it as documentation
//! with a compiler behind it, not an implementation detail — see that
//! module's doc comment.

pub mod field;

/// **Which of `checked`'s Field slots it actually evaluates**, in the order its
/// header declared them.
///
/// It used to be a yes-or-no — there was one field per Set and one spelling for
/// it, so "does this procedure call one" was the whole question. A slot is a
/// name now, and every consumer of this answer needs the name: a splice is
/// named for its slot, and so are the params it reads.
///
/// **Declared, filtered by the cost estimate**, which already walks every block
/// and counts the calls per slot — a second walk here would be a second answer
/// to one question, and this file has paid for that shape before. Declaration
/// order rather than first-call order, so that moving a call in a body does not
/// reorder a uniform struct.
///
/// **Only the slots it evaluates**, because a splice is a body in the caller's
/// module: a renderer that declares a field and never calls it would otherwise
/// carry its params and fail to compile if the field's body did — a `.kir`
/// taking down shaders that have nothing to do with it.
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

/// **Which `kind Field` procedure fills each Field slot a caller declared**, as
/// the Set resolved it: the caller's own name for the slot beside the procedure
/// bound to it.
///
/// **A list rather than one field**, because a Set holds as many as its edges
/// name. It used to be `Option<&Checked>` — "the Set's field, if it has one" —
/// which is the same rule the slot notation exists to remove, said in a
/// signature instead of in the language: a caller with two slots would have
/// reached one procedure through both however the edges were written.
///
/// **Resolved by the Set and never here.** An edge is a name on each end and
/// this crate has no names, so what arrives is already the answer; a slot with
/// no entry is one that was never called, since a declared slot nothing binds
/// is refused before any shader is generated.
pub type Bound<'a> = &'a [(&'a str, &'a Checked)];

/// The splices one caller needs: each bound field's body, once per slot the
/// caller reaches it through.
///
/// One function per *slot* rather than per field, because the name at the call
/// site is the caller's own. Two slots on one field are two identical bodies
/// under two names in one module, which costs a few hundred bytes of WGSL and
/// buys each of them its own params.
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
pub mod l1;
pub mod l2;
pub mod l3;
pub mod l4;
pub mod layout;
mod lower;
mod prelude;
mod ty;

pub use l1::{generate_l1, L1Shader};
pub use l2::{generate_l2, L2Shader};
pub use l3::{generate_l3, L3Shader};
pub use l4::{generate_l4, L4Shader};

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
}

/// Lowers a checked procedure to WGSL, picking the L1 or L4 path by
/// `checked.kind`. Prefer [`generate_l1`] / [`generate_l4`] directly when
/// the kind is already known statically.
///
/// `elements` is the paired L1 procedure's [`ElementLayout`] — required on
/// the `Kind::L4` path, since `generate_l4` compiles against it rather than
/// deriving its own (see that function's doc). `None` there panics; the L1
/// path ignores the argument, since `generate_l1` computes its own layout
/// from `checked.emit`.
pub fn generate(checked: &Checked, elements: Option<&ElementLayout>) -> Shader {
    match checked.kind {
        Kind::L1 => Shader::L1(generate_l1(checked, &[], &[])),
        // **Not reachable through this entry point.** An L2 is generated
        // against the attributes available *where it sits* in a chain, which is
        // a list rather than one upstream layout — `Set::build` has it and this
        // signature does not. Call `generate_l2` directly.
        Kind::L2 => panic!("an L2 is generated against its position in a chain: call generate_l2"),
        // **Not reachable through this entry point, and unlike an L2 it has no
        // entry point of its own.** A field lowers to a WGSL *function* spliced
        // into whichever procedures evaluate it, so it has no module, no
        // bindings and no dispatch — there is nothing for a `Shader` to hold.
        Kind::Field => panic!("a field lowers into its callers: call generate_field"),
        Kind::L3 => Shader::L3(generate_l3(checked, &[])),
        Kind::L4 => {
            let elements = elements.expect("an L4 procedure needs its paired L1's ElementLayout");
            Shader::L4(generate_l4(checked, elements, &[]))
        }
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
        // position is consumed but never read in fragment, so it must not
        // become a varying. Asked of the *varying* rather than of the whole
        // module: the `Element` struct declares `position: vec3<f32>` now that
        // a slot is its attribute's own width, and a bare substring search
        // finds that instead — which is the assertion passing for a reason
        // that has nothing to do with what it is checking.
        assert!(!src.contains(") position: vec3<f32>,"), "{src}");
        // point_coord is fragment-only ambient and IS used in fragment.
        assert!(src.contains("in.point_coord"), "{src}");
    }
}
