//! Tests for Structured Codegen AST and Pass Fusion (L2 + L4).

use karakuri_codegen::ast::{
    AddressSpace, BinaryOp, BindingDef, EntryPoint, Expr, FnParam, FunctionDef, ShaderModule,
    Stage, Stmt, StructDef, StructMember, WgslType,
};
use karakuri_codegen::fuse_l2_into_l4;
use karakuri_ir::builtin::Builtin;
use karakuri_ir::layout::{generate_element_layout, Synthetic};
use karakuri_ir::typed::{Checked, TBlock, TExpr, TExprKind, TStmt, Target};
use karakuri_ir::{
    Attr, BinOp, Blend, BlockKind, Kind, Lit as IrLit, Output, Param, Span, Topology, Ty,
};

fn span() -> Span {
    Span::EMPTY
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
fn shader_module_ast_constructs_valid_wgsl() {
    let mut module = ShaderModule::new();

    // struct Uniforms { time: f32, count: u32, _pad: vec2<f32>, }
    module.structs.push(StructDef {
        name: "Uniforms".to_string(),
        members: vec![
            StructMember {
                name: "time".to_string(),
                ty: WgslType::F32,
                attributes: vec![],
            },
            StructMember {
                name: "count".to_string(),
                ty: WgslType::U32,
                attributes: vec![],
            },
            StructMember {
                name: "_pad".to_string(),
                ty: WgslType::Vec2,
                attributes: vec![],
            },
        ],
    });

    // struct Element { position: vec3<f32>, age: f32, }
    module.structs.push(StructDef {
        name: "Element".to_string(),
        members: vec![
            StructMember {
                name: "position".to_string(),
                ty: WgslType::Vec3,
                attributes: vec![],
            },
            StructMember {
                name: "age".to_string(),
                ty: WgslType::F32,
                attributes: vec![],
            },
        ],
    });

    // @group(0) @binding(0) var<uniform> u: Uniforms;
    module.bindings.push(BindingDef {
        group: 0,
        binding: 0,
        name: "u".to_string(),
        ty: WgslType::Custom("Uniforms".to_string()),
        address_space: AddressSpace::Uniform,
    });

    // @group(1) @binding(0) var<storage, read_write> elements: array<Element>;
    module.bindings.push(BindingDef {
        group: 1,
        binding: 0,
        name: "elements".to_string(),
        ty: WgslType::Array(Box::new(WgslType::Custom("Element".to_string())), None),
        address_space: AddressSpace::StorageReadWrite,
    });

    // fn compute_wave(pos: vec3<f32>, t: f32) -> vec3<f32>
    module.functions.push(FunctionDef {
        name: "compute_wave".to_string(),
        params: vec![
            FnParam {
                name: "pos".to_string(),
                ty: WgslType::Vec3,
                attributes: vec![],
            },
            FnParam {
                name: "t".to_string(),
                ty: WgslType::F32,
                attributes: vec![],
            },
        ],
        return_type: Some(WgslType::Vec3),
        return_attributes: vec![],
        body: vec![
            Stmt::Let {
                name: "wave".to_string(),
                ty: Some(WgslType::F32),
                value: Expr::Call {
                    func: "sin".to_string(),
                    args: vec![Expr::Binary {
                        op: BinaryOp::Add,
                        lhs: Box::new(Expr::field(Expr::ident("pos"), "x")),
                        rhs: Box::new(Expr::ident("t")),
                    }],
                },
            },
            Stmt::Return(Some(Expr::Construct {
                ty: WgslType::Vec3,
                args: vec![
                    Expr::field(Expr::ident("pos"), "x"),
                    Expr::Binary {
                        op: BinaryOp::Add,
                        lhs: Box::new(Expr::field(Expr::ident("pos"), "y")),
                        rhs: Box::new(Expr::ident("wave")),
                    },
                    Expr::field(Expr::ident("pos"), "z"),
                ],
            })),
        ],
    });

    // @compute @workgroup_size(64, 1, 1) fn main_cs(@builtin(global_invocation_id) gid: vec3<u32>)
    module.entry_points.push(EntryPoint {
        stage: Stage::Compute {
            workgroup_size: [64, 1, 1],
        },
        name: "main_cs".to_string(),
        params: vec![FnParam {
            name: "gid".to_string(),
            ty: WgslType::Custom("vec3<u32>".to_string()),
            attributes: vec!["@builtin(global_invocation_id)".to_string()],
        }],
        return_type: None,
        return_attributes: vec![],
        body: vec![
            Stmt::Let {
                name: "idx".to_string(),
                ty: Some(WgslType::U32),
                value: Expr::field(Expr::ident("gid"), "x"),
            },
            Stmt::If {
                cond: Expr::Binary {
                    op: BinaryOp::GtEq,
                    lhs: Box::new(Expr::ident("idx")),
                    rhs: Box::new(Expr::field(Expr::ident("u"), "count")),
                },
                then_branch: vec![Stmt::Return(None)],
                else_branch: None,
            },
            Stmt::Let {
                name: "curr_pos".to_string(),
                ty: None,
                value: Expr::field(
                    Expr::index(Expr::ident("elements"), Expr::ident("idx")),
                    "position",
                ),
            },
            Stmt::Assign {
                target: Expr::field(
                    Expr::index(Expr::ident("elements"), Expr::ident("idx")),
                    "position",
                ),
                value: Expr::Call {
                    func: "compute_wave".to_string(),
                    args: vec![
                        Expr::ident("curr_pos"),
                        Expr::field(Expr::ident("u"), "time"),
                    ],
                },
            },
        ],
    });

    let wgsl = module.emit_wgsl();
    assert!(wgsl.contains("struct Uniforms"));
    assert!(wgsl.contains("fn compute_wave("));
    assert!(wgsl.contains("@compute @workgroup_size(64, 1, 1)"));

    validate(&wgsl);
}

fn empty_checked(name: &str, kind: Kind) -> Checked {
    Checked {
        name: name.to_string(),
        kind,
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        retains: false,
        blend: Some(Blend::Additive),
        params: Vec::new(),
        emit: Vec::new(),
        consumes: Vec::new(),
        blocks: Vec::new(),
        cost: None,
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

fn sample_l2_deform() -> Checked {
    let mut p = empty_checked("wave_deform", Kind::L2);
    p.params.push(Param {
        name: "speed".to_string(),
        ty: Ty::Float,
        min: 0.0,
        max: 5.0,
        default: karakuri_ir::Expr::Lit {
            value: IrLit::Float(1.0),
            span: span(),
        },
        span: span(),
    });

    // In deform block: position.y += sin(position.x)
    let pos_x = TExpr::new(
        Ty::Float,
        span(),
        TExprKind::Swizzle {
            components: vec![0],
            value: Box::new(TExpr::new(
                Ty::Vec3,
                span(),
                TExprKind::Attr(Attr::Position),
            )),
        },
    );
    let sin_val = TExpr::new(
        Ty::Float,
        span(),
        TExprKind::Builtin {
            func: Builtin::Sin,
            args: vec![pos_x],
        },
    );
    let pos_y = TExpr::new(
        Ty::Float,
        span(),
        TExprKind::Swizzle {
            components: vec![1],
            value: Box::new(TExpr::new(
                Ty::Vec3,
                span(),
                TExprKind::Attr(Attr::Position),
            )),
        },
    );
    let pos_z = TExpr::new(
        Ty::Float,
        span(),
        TExprKind::Swizzle {
            components: vec![2],
            value: Box::new(TExpr::new(
                Ty::Vec3,
                span(),
                TExprKind::Attr(Attr::Position),
            )),
        },
    );
    let new_y = TExpr::new(
        Ty::Float,
        span(),
        TExprKind::Binary {
            op: BinOp::Add,
            lhs: Box::new(pos_y),
            rhs: Box::new(sin_val),
        },
    );
    let new_pos = TExpr::new(
        Ty::Vec3,
        span(),
        TExprKind::Construct {
            args: vec![
                TExpr::new(
                    Ty::Float,
                    span(),
                    TExprKind::Swizzle {
                        components: vec![0],
                        value: Box::new(TExpr::new(
                            Ty::Vec3,
                            span(),
                            TExprKind::Attr(Attr::Position),
                        )),
                    },
                ),
                new_y,
                pos_z,
            ],
        },
    );

    let assign = TStmt::Assign {
        target: Target::Attr(Attr::Position),
        value: new_pos,
        span: span(),
    };

    p.blocks.push(TBlock {
        kind: BlockKind::Deform,
        stmts: vec![assign],
        span: span(),
    });
    p
}

fn sample_l4_renderer() -> Checked {
    let mut p = empty_checked("soft_points", Kind::L4);
    p.consumes = vec![Attr::Position];
    p.topology = Some(Topology::Points);
    p.blend = Some(Blend::Additive);

    p.params.push(Param {
        name: "point_size".to_string(),
        ty: Ty::Float,
        min: 0.001,
        max: 0.1,
        default: karakuri_ir::Expr::Lit {
            value: IrLit::Float(0.01),
            span: span(),
        },
        span: span(),
    });

    // Vertex block:
    // clip = vec4(position, 1.0);
    // point_rate = point_size;
    let clip_assign = TStmt::Assign {
        target: Target::Output(Output::Clip),
        value: TExpr::new(
            Ty::Vec4,
            span(),
            TExprKind::Construct {
                args: vec![
                    TExpr::new(Ty::Vec3, span(), TExprKind::Attr(Attr::Position)),
                    TExpr::new(Ty::Float, span(), TExprKind::Lit(IrLit::Float(1.0))),
                ],
            },
        ),
        span: span(),
    };
    let point_rate_assign = TStmt::Assign {
        target: Target::Output(Output::PointRate),
        value: TExpr::new(
            Ty::Float,
            span(),
            TExprKind::Param("point_size".to_string()),
        ),
        span: span(),
    };
    p.blocks.push(TBlock {
        kind: BlockKind::Vertex,
        stmts: vec![clip_assign, point_rate_assign],
        span: span(),
    });

    // Fragment block:
    // color = vec4(1.0, 0.5, 0.2, 1.0);
    let color_assign = TStmt::Assign {
        target: Target::Output(Output::Color),
        value: TExpr::new(
            Ty::Vec4,
            span(),
            TExprKind::Construct {
                args: vec![
                    TExpr::new(Ty::Float, span(), TExprKind::Lit(IrLit::Float(1.0))),
                    TExpr::new(Ty::Float, span(), TExprKind::Lit(IrLit::Float(0.5))),
                    TExpr::new(Ty::Float, span(), TExprKind::Lit(IrLit::Float(0.2))),
                    TExpr::new(Ty::Float, span(), TExprKind::Lit(IrLit::Float(1.0))),
                ],
            },
        ),
        span: span(),
    };
    p.blocks.push(TBlock {
        kind: BlockKind::Fragment,
        stmts: vec![color_assign],
        span: span(),
    });
    p
}

#[test]
fn pass_fusion_inlines_l2_deformation_into_l4_vertex_stage() {
    let l2 = sample_l2_deform();
    let l4 = sample_l4_renderer();
    let elements = generate_element_layout(&[Attr::Position], Synthetic::NONE, &[]);

    let fused = fuse_l2_into_l4(&l2, &l4, &elements, &[]);

    // 1. Verify inlined deformation function definition exists.
    assert!(
        fused
            .source
            .contains("fn deform_element(in_elem: Element, seed: u32) -> Element"),
        "expected deform_element function in WGSL:\n{}",
        fused.source
    );

    // 2. Verify vertex entry point loads element and invokes deform_element.
    assert!(
        fused.source.contains("var elem = elements[elem_idx];"),
        "expected element load in vertex entry point:\n{}",
        fused.source
    );
    assert!(
        fused.source.contains("elem = deform_element(elem, seed);"),
        "expected inlined deformation call in vertex entry point:\n{}",
        fused.source
    );

    // 3. Verify deformed element attributes feed vertex stage.
    assert!(
        fused.source.contains("let position = elem.position;"),
        "expected position attribute to be read from deformed elem:\n{}",
        fused.source
    );

    // 4. Verify uniform parameters from both L2 and L4 are safely namespaced.
    assert!(
        fused.source.contains("param_l2_speed"),
        "expected L2 param 'speed' to be namespaced as 'param_l2_speed':\n{}",
        fused.source
    );
    assert!(
        fused.source.contains("param_l4_point_size"),
        "expected L4 param 'point_size' to be namespaced as 'param_l4_point_size':\n{}",
        fused.source
    );

    // 5. Verify 16-byte alignment of uniform buffer.
    assert_eq!(
        fused.uniform_layout.total_size % 16,
        0,
        "fused uniform buffer total size must be 16-byte aligned"
    );

    // 6. Full Naga parsing and validation.
    validate(&fused.source);
}

#[test]
fn pass_fusion_with_mask_and_weight_compiles_and_validates() {
    let mut l2 = sample_l2_deform();
    l2.params.push(Param {
        name: "weight".to_string(),
        ty: Ty::Float,
        min: 0.0,
        max: 1.0,
        default: karakuri_ir::Expr::Lit {
            value: IrLit::Float(1.0),
            span: span(),
        },
        span: span(),
    });

    // Add mask block: strength = position.x;
    let pos_x = TExpr::new(
        Ty::Float,
        span(),
        TExprKind::Swizzle {
            components: vec![0],
            value: Box::new(TExpr::new(
                Ty::Vec3,
                span(),
                TExprKind::Attr(Attr::Position),
            )),
        },
    );
    l2.blocks.push(TBlock {
        kind: BlockKind::Mask,
        stmts: vec![TStmt::Assign {
            target: Target::Output(Output::Strength),
            value: pos_x,
            span: span(),
        }],
        span: span(),
    });

    let l4 = sample_l4_renderer();
    let elements = generate_element_layout(&[Attr::Position], Synthetic::NONE, &[]);

    let fused = fuse_l2_into_l4(&l2, &l4, &elements, &[]);

    assert!(fused.source.contains("param_l2_weight"));
    assert!(fused
        .source
        .contains("let _gate = clamp(strength, 0.0, 1.0);"));
    assert!(fused
        .source
        .contains("mix(_input.position, elem.position, _gate)"));

    validate(&fused.source);
}
