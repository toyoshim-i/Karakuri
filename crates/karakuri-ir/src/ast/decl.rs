use crate::span::Span;

use super::*;

/// Parameter declaration: `param <name> : <type> [<min>, <max>] = <default>`.
///
/// Defines a tunable fader/control input. For vector types, `[min, max]` applies per component.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Ty,
    pub min: f32,
    pub max: f32,
    pub default: Expr,
    pub span: Span,
}

impl Param {
    /// Returns the declared default value as a scalar float if statically foldable, or `None`.
    pub fn default_scalar(&self) -> Option<f32> {
        fold_literal(&self.default)
    }

    /// Returns the declared default value unfolded per vector component, or `None`.
    pub fn default_components(&self) -> Option<Vec<f32>> {
        let width = self.ty.param_components()?;
        if width == 1 {
            return fold_literal(&self.default).map(|v| vec![v]);
        }
        let Expr::Call { name, args, .. } = &self.default else {
            return None;
        };
        if name != self.ty.name() {
            return None;
        }
        match args.len() {
            1 => fold_literal(&args[0]).map(|v| vec![v; width]),
            n if n == width => args.iter().map(fold_literal).collect(),
            _ => None,
        }
    }

    /// Returns parameter key strings in component order (`x`, `y`, `z`).
    ///
    /// Returns `[name]` for scalar float, or `[name.x, name.y, ...]` for vectors.
    pub fn keys(&self) -> Vec<String> {
        match self.ty.param_components() {
            Some(width) if width > 1 => (0..width).map(|i| component_key(&self.name, i)).collect(),
            _ => vec![self.name.clone()],
        }
    }
}

/// Folds a float literal or negated float literal expression to `f32`.
fn fold_literal(expr: &Expr) -> Option<f32> {
    match expr {
        Expr::Lit {
            value: Lit::Float(v),
            ..
        } => Some(*v),
        Expr::Unary {
            op: UnOp::Neg,
            value,
            ..
        } => match value.as_ref() {
            Expr::Lit {
                value: Lit::Float(v),
                ..
            } => Some(-v),
            _ => None,
        },
        _ => None,
    }
}

/// Swizzle component suffixes used for vector parameter names (`x`, `y`, `z`).
pub const COMPONENTS: [&str; 3] = ["x", "y", "z"];

/// Returns the parameter key for a vector component (e.g. `glow.y` for component 1 of `glow`).
pub fn component_key(name: &str, component: usize) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    push_component_key(&mut out, name, component);
    out
}

/// Appends a vector parameter component key into the given string buffer.
pub fn push_component_key(out: &mut String, name: &str, component: usize) {
    out.push_str(name);
    out.push('.');
    out.push_str(COMPONENTS[component]);
}

/// `capacity [<min>, <max>] = <default>`
///
/// A range rather than a constant, because a Set overrides the value and the
/// artifact has to declare what it can be turned to.
#[derive(Debug, Clone, Copy)]
pub struct CapacityDecl {
    pub min: u32,
    pub max: u32,
    pub default: u32,
    pub span: Span,
}

impl CapacityDecl {
    pub fn contains(&self, value: u32) -> bool {
        (self.min..=self.max).contains(&value)
    }
}

/// Default capacity limit per geometry when unspecified (262,144 elements).
pub const DEFAULT_CAPACITY: u32 = 262_144;

/// Amplification declaration: `amplify <factor>`.
#[derive(Debug, Clone, Copy)]
pub struct AmplifyDecl {
    pub factor: u32,
    pub span: Span,
}

/// Declaration of a named input slot: `uses <name> : <SlotTy>`.
///
/// Defines an input dependency bound at Set composition time (ADR-0283).
#[derive(Debug, Clone)]
pub struct UsesDecl {
    pub name: String,
    pub name_span: Span,
    pub ty: SlotTy,
    pub span: Span,
}

/// Retained frame input declaration: `retains`.
///
/// Indicates that an L5 procedure reads the previous frame via `held`.
#[derive(Debug, Clone, Copy)]
pub struct RetainsDecl {
    pub span: Span,
}

/// One `proc`, which is one file and fills one slot.
#[derive(Debug, Clone)]
pub struct Proc {
    pub name: String,
    pub kind: Kind,
    /// L1 only.
    pub topology: Option<Topology>,
    /// L1 only.
    pub capacity: Option<CapacityDecl>,
    /// L2 only.
    pub amplify: Option<AmplifyDecl>,
    /// L5 only — see [`RetainsDecl`].
    pub retains: Option<RetainsDecl>,
    /// Named input slots declared by this procedure.
    ///
    /// Slot type constraints are validated during contract checking ([`check_header`]).
    pub uses: Vec<UsesDecl>,
    /// L4 only.
    pub blend: Option<Blend>,
    pub params: Vec<Param>,
    pub emit: Vec<(Attr, Span)>,
    pub consumes: Vec<(Attr, Span)>,
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl Proc {
    pub fn block(&self, kind: BlockKind) -> Option<&Block> {
        self.blocks.iter().find(|b| b.kind == kind)
    }

    pub fn param(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|p| p.name == name)
    }

    /// The spawn rate parameter, which the engine reads specially.
    pub fn spawn_rate(&self) -> Option<&Param> {
        self.param("spawn_rate")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockKind {
    /// L1: initialise a newly allocated element.
    Spawn,
    /// L1: update a live element. May call `kill()`.
    Element,
    /// L2: rewrite a live element's attributes. May not `kill()` — see
    /// [`Kind::L2`].
    Deform,
    /// L2: where this deformation applies, and how much. Optional; an L2 with no
    /// `mask` block applies everywhere at full strength.
    Mask,
    /// L3: produce this frame's camera state.
    Camera,
    /// Field: the signed distance at [`Ambient::Point`]. The whole of a field
    /// procedure, and the only block whose lowering is a WGSL function rather than
    /// an entry point.
    Field,
    /// L4: once per element.
    Vertex,
    /// L4: once per rasterised fragment.
    Fragment,
    /// L5: once per texel of the input frame.
    Frame,
}

impl BlockKind {
    pub fn name(self) -> &'static str {
        match self {
            BlockKind::Spawn => "spawn",
            BlockKind::Element => "element",
            BlockKind::Deform => "deform",
            BlockKind::Mask => "mask",
            BlockKind::Camera => "camera",
            BlockKind::Vertex => "vertex",
            BlockKind::Fragment => "fragment",
            BlockKind::Field => "field",
            BlockKind::Frame => "frame",
        }
    }

    pub fn from_name(s: &str) -> Option<BlockKind> {
        Some(match s {
            "spawn" => BlockKind::Spawn,
            "element" => BlockKind::Element,
            "deform" => BlockKind::Deform,
            "mask" => BlockKind::Mask,
            "camera" => BlockKind::Camera,
            "vertex" => BlockKind::Vertex,
            "fragment" => BlockKind::Fragment,
            "frame" => BlockKind::Frame,
            _ => return None,
        })
    }

    pub fn kind(self) -> Kind {
        match self {
            BlockKind::Spawn | BlockKind::Element => Kind::L1,
            BlockKind::Deform | BlockKind::Mask => Kind::L2,
            BlockKind::Camera => Kind::L3,
            BlockKind::Vertex | BlockKind::Fragment => Kind::L4,
            BlockKind::Field => Kind::Field,
            BlockKind::Frame => Kind::L5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    pub stmts: Vec<Stmt>,
    pub span: Span,
}
