//! Structured WGSL Codegen AST.
//!
//! Provides a typed intermediate representation for generating WGSL shaders
//! directly from structured AST nodes rather than raw string templating.

use std::fmt::{self, Display, Formatter};

/// Primitive and composite WGSL types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WgslType {
    F32,
    U32,
    I32,
    Bool,
    Vec2,
    Vec3,
    Vec4,
    Mat4,
    Custom(String),
    Array(Box<WgslType>, Option<usize>),
}

impl Display for WgslType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::F32 => write!(f, "f32"),
            Self::U32 => write!(f, "u32"),
            Self::I32 => write!(f, "i32"),
            Self::Bool => write!(f, "bool"),
            Self::Vec2 => write!(f, "vec2<f32>"),
            Self::Vec3 => write!(f, "vec3<f32>"),
            Self::Vec4 => write!(f, "vec4<f32>"),
            Self::Mat4 => write!(f, "mat4x4<f32>"),
            Self::Custom(name) => write!(f, "{name}"),
            Self::Array(elem, Some(len)) => write!(f, "array<{elem}, {len}>"),
            Self::Array(elem, None) => write!(f, "array<{elem}>"),
        }
    }
}

/// Literals supported in WGSL.
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    Float(f32),
    Uint(u32),
    Int(i32),
    Bool(bool),
}

impl Display for Lit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Float(val) => {
                if val.is_nan() {
                    write!(f, "(0.0 / 0.0)")
                } else if val.is_infinite() {
                    if *val > 0.0 {
                        write!(f, "(1.0 / 0.0)")
                    } else {
                        write!(f, "(-1.0 / 0.0)")
                    }
                } else if val.fract() == 0.0 {
                    write!(f, "{val:.1}")
                } else {
                    write!(f, "{val}")
                }
            }
            Self::Uint(val) => write!(f, "{val}u"),
            Self::Int(val) => write!(f, "{val}"),
            Self::Bool(val) => write!(f, "{val}"),
        }
    }
}

/// Binary operators in WGSL expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Rem => write!(f, "%"),
            Self::Eq => write!(f, "=="),
            Self::NotEq => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::LtEq => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::GtEq => write!(f, ">="),
            Self::And => write!(f, "&&"),
            Self::Or => write!(f, "||"),
            Self::BitAnd => write!(f, "&"),
            Self::BitOr => write!(f, "|"),
            Self::BitXor => write!(f, "^"),
            Self::Shl => write!(f, "<<"),
            Self::Shr => write!(f, ">>"),
        }
    }
}

/// Unary operators in WGSL expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Neg => write!(f, "-"),
            Self::Not => write!(f, "!"),
        }
    }
}

/// Typed expression tree in WGSL.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Lit(Lit),
    Ident(String),
    FieldAccess {
        base: Box<Expr>,
        field: String,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        func: String,
        args: Vec<Expr>,
    },
    Construct {
        ty: WgslType,
        args: Vec<Expr>,
    },
}

impl Expr {
    pub fn ident(name: impl Into<String>) -> Self {
        Self::Ident(name.into())
    }

    pub fn float(val: f32) -> Self {
        Self::Lit(Lit::Float(val))
    }

    pub fn uint(val: u32) -> Self {
        Self::Lit(Lit::Uint(val))
    }

    pub fn int(val: i32) -> Self {
        Self::Lit(Lit::Int(val))
    }

    pub fn bool(val: bool) -> Self {
        Self::Lit(Lit::Bool(val))
    }

    pub fn field(base: Expr, field: impl Into<String>) -> Self {
        Self::FieldAccess {
            base: Box::new(base),
            field: field.into(),
        }
    }

    pub fn index(base: Expr, index: Expr) -> Self {
        Self::Index {
            base: Box::new(base),
            index: Box::new(index),
        }
    }

    pub fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Self {
        Self::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn unary(op: UnaryOp, expr: Expr) -> Self {
        Self::Unary {
            op,
            expr: Box::new(expr),
        }
    }

    pub fn call(func: impl Into<String>, args: Vec<Expr>) -> Self {
        Self::Call {
            func: func.into(),
            args,
        }
    }

    pub fn construct(ty: WgslType, args: Vec<Expr>) -> Self {
        Self::Construct { ty, args }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lit(lit) => write!(f, "{lit}"),
            Self::Ident(name) => write!(f, "{name}"),
            Self::FieldAccess { base, field } => write!(f, "{base}.{field}"),
            Self::Index { base, index } => write!(f, "{base}[{index}]"),
            Self::Binary { op, lhs, rhs } => write!(f, "({lhs} {op} {rhs})"),
            Self::Unary { op, expr } => write!(f, "({op}{expr})"),
            Self::Call { func, args } => {
                write!(f, "{func}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ")")
            }
            Self::Construct { ty, args } => {
                write!(f, "{ty}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Statements in WGSL function and entry point bodies.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<WgslType>,
        value: Expr,
    },
    Var {
        name: String,
        ty: Option<WgslType>,
        value: Option<Expr>,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    If {
        cond: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        update: Option<Box<Stmt>>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Discard,
    Expr(Expr),
    Block(Vec<Stmt>),
}

impl Stmt {
    fn format_indented(&self, f: &mut Formatter<'_>, indent: usize) -> fmt::Result {
        let pad = "    ".repeat(indent);
        match self {
            Self::Let { name, ty, value } => {
                if let Some(t) = ty {
                    writeln!(f, "{pad}let {name}: {t} = {value};")
                } else {
                    writeln!(f, "{pad}let {name} = {value};")
                }
            }
            Self::Var { name, ty, value } => match (ty, value) {
                (Some(t), Some(val)) => writeln!(f, "{pad}var {name}: {t} = {val};"),
                (Some(t), None) => writeln!(f, "{pad}var {name}: {t};"),
                (None, Some(val)) => writeln!(f, "{pad}var {name} = {val};"),
                (None, None) => writeln!(f, "{pad}var {name};"),
            },
            Self::Assign { target, value } => {
                writeln!(f, "{pad}{target} = {value};")
            }
            Self::If {
                cond,
                then_branch,
                else_branch,
            } => {
                writeln!(f, "{pad}if {cond} {{")?;
                for stmt in then_branch {
                    stmt.format_indented(f, indent + 1)?;
                }
                if let Some(else_stmts) = else_branch {
                    writeln!(f, "{pad}}} else {{")?;
                    for stmt in else_stmts {
                        stmt.format_indented(f, indent + 1)?;
                    }
                }
                writeln!(f, "{pad}}}")
            }
            Self::For {
                init,
                cond,
                update,
                body,
            } => {
                write!(f, "{pad}for (")?;
                if let Some(i) = init {
                    let s = format!("{i}");
                    let trimmed = s.trim().trim_end_matches(';');
                    write!(f, "{trimmed}; ")?;
                } else {
                    write!(f, "; ")?;
                }
                if let Some(c) = cond {
                    write!(f, "{c}; ")?;
                } else {
                    write!(f, "; ")?;
                }
                if let Some(u) = update {
                    let s = format!("{u}");
                    let trimmed = s.trim().trim_end_matches(';');
                    write!(f, "{trimmed}")?;
                }
                writeln!(f, ") {{")?;
                for stmt in body {
                    stmt.format_indented(f, indent + 1)?;
                }
                writeln!(f, "{pad}}}")
            }
            Self::Return(Some(val)) => writeln!(f, "{pad}return {val};"),
            Self::Return(None) => writeln!(f, "{pad}return;"),
            Self::Discard => writeln!(f, "{pad}discard;"),
            Self::Expr(expr) => writeln!(f, "{pad}{expr};"),
            Self::Block(stmts) => {
                writeln!(f, "{pad}{{")?;
                for stmt in stmts {
                    stmt.format_indented(f, indent + 1)?;
                }
                writeln!(f, "{pad}}}")
            }
        }
    }
}

impl Display for Stmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.format_indented(f, 0)
    }
}

/// Member of a WGSL struct declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct StructMember {
    pub name: String,
    pub ty: WgslType,
    pub attributes: Vec<String>,
}

impl Display for StructMember {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for attr in &self.attributes {
            write!(f, "{attr} ")?;
        }
        write!(f, "{}: {}", self.name, self.ty)
    }
}

/// A WGSL struct declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub members: Vec<StructMember>,
}

impl Display for StructDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "struct {} {{", self.name)?;
        for member in &self.members {
            writeln!(f, "    {member},")?;
        }
        writeln!(f, "}};")
    }
}

/// Address space and access mode for resource bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpace {
    Uniform,
    StorageRead,
    StorageReadWrite,
}

impl Display for AddressSpace {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uniform => write!(f, "uniform"),
            Self::StorageRead => write!(f, "storage, read"),
            Self::StorageReadWrite => write!(f, "storage, read_write"),
        }
    }
}

/// A resource binding declaration (`@group(g) @binding(b) var<...> name: ty;`).
#[derive(Debug, Clone, PartialEq)]
pub struct BindingDef {
    pub group: u32,
    pub binding: u32,
    pub name: String,
    pub ty: WgslType,
    pub address_space: AddressSpace,
}

impl Display for BindingDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "@group({}) @binding({}) var<{}> {}: {};",
            self.group, self.binding, self.address_space, self.name, self.ty
        )
    }
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct FnParam {
    pub name: String,
    pub ty: WgslType,
    pub attributes: Vec<String>,
}

impl Display for FnParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for attr in &self.attributes {
            write!(f, "{attr} ")?;
        }
        write!(f, "{}: {}", self.name, self.ty)
    }
}

/// A helper function definition.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<FnParam>,
    pub return_type: Option<WgslType>,
    pub return_attributes: Vec<String>,
    pub body: Vec<Stmt>,
}

impl Display for FunctionDef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}(", self.name)?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{param}")?;
        }
        write!(f, ")")?;
        if let Some(ret_ty) = &self.return_type {
            write!(f, " -> ")?;
            for attr in &self.return_attributes {
                write!(f, "{attr} ")?;
            }
            write!(f, "{ret_ty}")?;
        }
        writeln!(f, " {{")?;
        for stmt in &self.body {
            stmt.format_indented(f, 1)?;
        }
        writeln!(f, "}}")
    }
}

/// Shader pipeline execution stage.
#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    Compute { workgroup_size: [u32; 3] },
    Vertex,
    Fragment,
}

/// An entry point definition (`@compute`, `@vertex`, or `@fragment`).
#[derive(Debug, Clone, PartialEq)]
pub struct EntryPoint {
    pub stage: Stage,
    pub name: String,
    pub params: Vec<FnParam>,
    pub return_type: Option<WgslType>,
    pub return_attributes: Vec<String>,
    pub body: Vec<Stmt>,
}

impl Display for EntryPoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.stage {
            Stage::Compute { workgroup_size } => {
                writeln!(
                    f,
                    "@compute @workgroup_size({}, {}, {})",
                    workgroup_size[0], workgroup_size[1], workgroup_size[2]
                )?;
            }
            Stage::Vertex => {
                writeln!(f, "@vertex")?;
            }
            Stage::Fragment => {
                writeln!(f, "@fragment")?;
            }
        }
        write!(f, "fn {}(", self.name)?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{param}")?;
        }
        write!(f, ")")?;
        if let Some(ret_ty) = &self.return_type {
            write!(f, " -> ")?;
            for attr in &self.return_attributes {
                write!(f, "{attr} ")?;
            }
            write!(f, "{ret_ty}")?;
        }
        writeln!(f, " {{")?;
        for stmt in &self.body {
            stmt.format_indented(f, 1)?;
        }
        writeln!(f, "}}")
    }
}

/// A complete structured WGSL shader module.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShaderModule {
    pub structs: Vec<StructDef>,
    pub bindings: Vec<BindingDef>,
    pub functions: Vec<FunctionDef>,
    pub entry_points: Vec<EntryPoint>,
}

impl ShaderModule {
    pub fn new() -> Self {
        Self::default()
    }

    /// Emits cleanly formatted WGSL text representation of the module.
    pub fn emit_wgsl(&self) -> String {
        self.to_string()
    }
}

impl Display for ShaderModule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for struct_def in &self.structs {
            writeln!(f, "{struct_def}")?;
        }
        for binding in &self.bindings {
            writeln!(f, "{binding}")?;
        }
        for func in &self.functions {
            writeln!(f, "{func}")?;
        }
        for ep in &self.entry_points {
            writeln!(f, "{ep}")?;
        }
        Ok(())
    }
}
