//! Typed and resolved intermediate representation produced by the check pass.

use crate::ast::{
    Ambient, Attr, BinOp, Blend, BlockKind, CapacityDecl, Kind, Lit, Output, Param, SlotTy,
    Topology, Ty,
};
use crate::builtin::Builtin;
use crate::span::Span;

/// Identifier of an input dependency port bound via `uses` declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputPort(pub String);

impl InputPort {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for InputPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for InputPort {
    fn from(name: String) -> InputPort {
        InputPort(name)
    }
}

impl From<&str> for InputPort {
    fn from(name: &str) -> InputPort {
        InputPort(name.to_string())
    }
}

/// Resolved input slot declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub name: InputPort,
    pub ty: SlotTy,
}

/// Resolved texture reference in an L5 procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TexRef {
    /// `src`: incoming texture input.
    Src,
    /// `held`: retained cut from previous frame.
    Held,
    /// Named `uses <name> : Texture` slot.
    Slot(InputPort),
}

/// A procedure that has passed parsing, type checking, and contract checking.
#[derive(Debug, Clone)]
pub struct Checked {
    pub name: String,
    pub kind: Kind,
    /// Geometry topology (declared for L1, inferred for L4).
    pub topology: Option<Topology>,
    pub capacity: Option<CapacityDecl>,
    /// Element amplification factor for amplifying L2 deforms.
    pub amplify: Option<u32>,
    /// Declared input dependency slots.
    pub uses: Vec<Slot>,
    /// Whether this procedure reads the retained frame (`retains`).
    pub retains: bool,
    pub blend: Option<Blend>,
    pub params: Vec<Param>,
    pub emit: Vec<Attr>,
    pub consumes: Vec<Attr>,
    pub blocks: Vec<TBlock>,
    /// Cost estimation profile, if computed.
    pub cost: Option<Cost>,
    /// True when state at time `t` is a pure function of inputs (no simulation history).
    pub closed_form: bool,
    /// True if the procedure reads musical beat position (`beats`).
    pub reads_beats: bool,
    pub span: Span,
}

impl Checked {
    pub fn block(&self, kind: BlockKind) -> Option<&TBlock> {
        self.blocks.iter().find(|b| b.kind == kind)
    }

    /// Returns the declared geometry slot name, if any.
    pub fn geometry_slot(&self) -> Option<&str> {
        self.uses
            .iter()
            .find(|s| s.ty == SlotTy::Geometry)
            .map(|s| s.name.as_str())
    }

    /// Returns declared field slot names in header order.
    pub fn field_slots(&self) -> Vec<&str> {
        self.uses
            .iter()
            .filter(|s| s.ty == SlotTy::Field)
            .map(|s| s.name.as_str())
            .collect()
    }

    /// Returns the declared camera slot name, or `None` if drawing with default camera.
    pub fn camera_slot(&self) -> Option<&str> {
        self.uses
            .iter()
            .find(|s| s.ty == SlotTy::Camera)
            .map(|s| s.name.as_str())
    }

    /// Returns declared texture slot names in header order.
    pub fn texture_slots(&self) -> Vec<&str> {
        self.uses
            .iter()
            .filter(|s| s.ty == SlotTy::Texture)
            .map(|s| s.name.as_str())
            .collect()
    }

    /// Returns declared source slot names in header order.
    pub fn source_slots(&self) -> Vec<&str> {
        self.uses
            .iter()
            .filter(|s| s.ty == SlotTy::Source)
            .map(|s| s.name.as_str())
            .collect()
    }

    /// Returns true if elements never spawn or die, leaving element indices static.
    pub fn is_static(&self) -> bool {
        self.kind == Kind::L1
            && self.block(BlockKind::Spawn).is_none()
            && !self.blocks.iter().any(|b| contains_kill(&b.stmts))
    }
}

/// Returns true if any statement in `stmts` can kill an element.
pub fn contains_kill(stmts: &[TStmt]) -> bool {
    stmts.iter().any(|s| match s {
        TStmt::Kill { .. } => true,
        TStmt::If { then, els, .. } => contains_kill(then) || contains_kill(els),
        TStmt::For { body, .. } => contains_kill(body),
        TStmt::Let { .. } | TStmt::Var { .. } | TStmt::Assign { .. } => false,
    })
}

/// Tracks field call counts for a specific slot across stages.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SlotCalls {
    pub slot: String,
    pub per_element: u64,
    pub per_spawn: u64,
    pub per_fragment: u64,
    pub total: u64,
}

/// Aggregated field evaluation counts grouped by slot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldCalls {
    pub slots: Vec<SlotCalls>,
}

impl FieldCalls {
    /// Returns true if any field evaluations occur across any block.
    pub fn any(&self) -> bool {
        self.slots.iter().any(|s| s.total > 0)
    }

    /// Returns field evaluation counts for the given slot, or `None`.
    pub fn slot(&self, slot: &str) -> Option<&SlotCalls> {
        self.slots.iter().find(|s| s.slot == slot)
    }

    pub(crate) fn entry(&mut self, slot: &str) -> &mut SlotCalls {
        if let Some(at) = self.slots.iter().position(|s| s.slot == slot) {
            return &mut self.slots[at];
        }
        self.slots.push(SlotCalls {
            slot: slot.to_string(),
            ..SlotCalls::default()
        });
        self.slots.last_mut().expect("just pushed")
    }
}

/// Static instruction cost estimates for pipeline stages and field calls.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cost {
    pub ops_per_element: u64,
    pub ops_per_spawn: u64,
    pub ops_per_fragment: u64,
    pub ops_per_evaluation: u64,
    pub field_calls: FieldCalls,
}

#[derive(Debug, Clone)]
pub struct TBlock {
    pub kind: BlockKind,
    pub stmts: Vec<TStmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TStmt {
    /// Locals do not shadow anything — not a param, not an attribute, not an
    /// ambient, and not another local — so a name identifies one binding within a
    /// block and code generation can emit it verbatim.
    Let {
        name: String,
        value: TExpr,
        span: Span,
    },
    Var {
        name: String,
        value: TExpr,
        span: Span,
    },
    /// Always simple assignment: compound forms are desugared during checking.
    Assign {
        target: Target,
        value: TExpr,
        span: Span,
    },
    If {
        cond: TExpr,
        then: Vec<TStmt>,
        els: Vec<TStmt>,
        span: Span,
    },
    For {
        var: String,
        start: i32,
        end: i32,
        body: Vec<TStmt>,
        span: Span,
    },
    Kill {
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Local(String),
    /// Writes the next frame's buffer. Reading the same attribute anywhere in the
    /// block still yields the previous frame's value.
    Attr(Attr),
    Output(Output),
}

#[derive(Debug, Clone)]
pub struct TExpr {
    pub ty: Ty,
    pub span: Span,
    pub kind: TExprKind,
}

#[derive(Debug, Clone)]
pub enum TExprKind {
    Lit(Lit),
    Local(String),
    Param(String),
    /// A read of the previous frame's element attribute.
    Attr(Attr),
    /// A read of the far element's attribute in a dual-geometry L2 deformation (`<slot>.<name>`).
    Far(Attr),
    Ambient(Ambient),
    Unary {
        op: crate::ast::UnOp,
        value: Box<TExpr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<TExpr>,
        rhs: Box<TExpr>,
    },
    /// A builtin with its overload already chosen: `ty` is the resolved result
    /// type, so the generator never re-unifies.
    Builtin {
        func: Builtin,
        args: Vec<TExpr>,
    },
    /// Evaluation of a field at a given point: `<slot>(p)`.
    ///
    /// Lowers to a call of the bound field function identified by slot name.
    Field {
        slot: String,
        point: Box<TExpr>,
    },
    /// Identity of the geometry bound to a declared Source slot.
    ///
    /// Resolves to the source instance index (uint) bound to the named slot.
    Source {
        slot: String,
    },
    /// A fetch from a texture input in an L5 procedure (`texel` or `tap`).
    Sample {
        func: Builtin,
        texture: TexRef,
        /// Frame UV coordinate for `tap`, or `None` for `texel`.
        at: Option<Box<TExpr>>,
    },
    /// `vec3(a, b, c)`, `vec3(x)` broadcasting, and the scalar conversions
    /// `float(i)` / `int(x)` / `uint(x)`. All of them construct `ty` from `args`;
    /// there are no implicit conversions anywhere else, so a cast in the generated
    /// WGSL corresponds to one of these and nothing else.
    Construct {
        args: Vec<TExpr>,
    },
    /// `v.xy`, `v.zyx`. Components are 0-based indices, already validated against
    /// the source width.
    Swizzle {
        value: Box<TExpr>,
        components: Vec<u8>,
    },
}

impl TExpr {
    pub fn new(ty: Ty, span: Span, kind: TExprKind) -> TExpr {
        TExpr { ty, span, kind }
    }
}
