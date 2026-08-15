//! The `.kir` abstract syntax tree.
//!
//! The parser produces this verbatim from source. Names are not resolved, types
//! are not inferred, and nothing beyond syntax is checked: an `Assign` target is
//! a bare string here, and a `Call` covers builtins and type constructors alike.
//! Resolution and typing happen in the check pass, which consumes this.
//!
//! Specification: `docs/ir-spec.md`.

use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Geometry generation. Stateful, compute.
    L1,
    /// Rendering. Stateless, a render pipeline.
    L4,
}

/// What a procedure's geometry *is*, on an L1 header, and what an L4 procedure
/// *draws*, inferred rather than declared — see [`crate::typed::Checked::topology`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topology {
    /// One sprite per element.
    Points,
    /// **One segment per element**, from [`Output::Clip`] to [`Output::ClipB`].
    ///
    /// The connectivity is deliberately not a graph. An element nominating
    /// *another element* as its far end would be an index into the element
    /// buffer, and compaction moves elements between steps — so the one shape
    /// that would express a strip is also the one that spawning material
    /// invalidates. A segment whose two ends both belong to one element has
    /// no such dependency: a polyline is *n* segments, a trail is one segment
    /// per particle, and both survive compaction because neither refers to
    /// anything outside itself.
    ///
    /// It costs the duplication of shared endpoints — a strip of *n* samples
    /// stores 2*n* points rather than *n*+1. That is the price of the
    /// primitive being smaller than the gesture.
    Lines,
    /// **The whole frame, once, with no geometry at all.**
    ///
    /// Inferred for an L4 that has no `vertex` block, because a procedure with
    /// no per-element position has nothing for one to do. **Refused on an L1**:
    /// geometry cannot be fullscreen, and the value exists on this enum only
    /// because an L4's answer and an L1's declaration share a field.
    ///
    /// Such a procedure consumes nothing — there is nowhere to read an element
    /// from — which is what lets the engine skip the paired L1's simulation
    /// entirely rather than run it for a reader that does not exist.
    Fullscreen,
}

impl Topology {
    pub fn name(self) -> &'static str {
        match self {
            Topology::Points => "points",
            Topology::Lines => "lines",
            Topology::Fullscreen => "fullscreen",
        }
    }
}

/// The v0.2 grammar admits exactly one value. The declaration exists so that
/// weighted blended OIT is a later addition rather than a format change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    Additive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    Float,
    Int,
    Uint,
    Bool,
    Vec2,
    Vec3,
    Vec4,
    Mat3,
    Mat4,
}

impl Ty {
    pub fn name(self) -> &'static str {
        match self {
            Ty::Float => "float",
            Ty::Int => "int",
            Ty::Uint => "uint",
            Ty::Bool => "bool",
            Ty::Vec2 => "vec2",
            Ty::Vec3 => "vec3",
            Ty::Vec4 => "vec4",
            Ty::Mat3 => "mat3",
            Ty::Mat4 => "mat4",
        }
    }

    pub fn from_name(s: &str) -> Option<Ty> {
        Some(match s {
            "float" => Ty::Float,
            "int" => Ty::Int,
            "uint" => Ty::Uint,
            "bool" => Ty::Bool,
            "vec2" => Ty::Vec2,
            "vec3" => Ty::Vec3,
            "vec4" => Ty::Vec4,
            "mat3" => Ty::Mat3,
            "mat4" => Ty::Mat4,
            _ => return None,
        })
    }

    /// Component count for vectors, 1 for scalars, `None` for matrices.
    pub fn components(self) -> Option<u8> {
        Some(match self {
            Ty::Float | Ty::Int | Ty::Uint | Ty::Bool => 1,
            Ty::Vec2 => 2,
            Ty::Vec3 => 3,
            Ty::Vec4 => 4,
            Ty::Mat3 | Ty::Mat4 => return None,
        })
    }
}

/// Attributes an L1 procedure emits and an L4 procedure consumes.
///
/// `seed` is deliberately absent. It is implicit, always allocated, and never
/// declared in `emit` or `consumes` — see `Ambient::Seed` and the element
/// identity section of the spec. `id` does not exist at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Attr {
    Position,
    Velocity,
    Normal,
    Uv,
    Age,
    Size,
    Tint,
}

impl Attr {
    pub const ALL: [Attr; 7] = [
        Attr::Position,
        Attr::Velocity,
        Attr::Normal,
        Attr::Uv,
        Attr::Age,
        Attr::Size,
        Attr::Tint,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Attr::Position => "position",
            Attr::Velocity => "velocity",
            Attr::Normal => "normal",
            Attr::Uv => "uv",
            Attr::Age => "age",
            Attr::Size => "size",
            Attr::Tint => "tint",
        }
    }

    pub fn from_name(s: &str) -> Option<Attr> {
        Attr::ALL.into_iter().find(|a| a.name() == s)
    }

    pub fn ty(self) -> Ty {
        match self {
            Attr::Position | Attr::Velocity | Attr::Normal | Attr::Tint => Ty::Vec3,
            Attr::Uv => Ty::Vec2,
            Attr::Age | Attr::Size => Ty::Float,
        }
    }

    /// Whether `docs/ir-spec.md` describes a rule for synthesising this
    /// attribute when a consumer needs it and the producer does not emit it.
    /// No such rule is implemented — see "Beyond v0.2 — specified, not
    /// implemented" — so this does not mean the check pass will accept a
    /// `consumes` entry missing from `emit`; it only changes the diagnostic
    /// `check_consumes_emitted` produces when it rejects one, so a
    /// regenerating model is told the rule exists rather than told the spec
    /// is wrong.
    pub fn is_derivable(self) -> bool {
        matches!(self, Attr::Velocity | Attr::Age)
    }
}

/// Stage outputs. Written with attribute syntax; reading one is an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Output {
    /// vertex: clip-space position. A sprite's centre, or a segment's near end.
    Clip,
    /// vertex: **a segment's far end**, in clip space.
    ///
    /// Optional, and the only optional output. Assigning it is how an L4
    /// procedure says it draws [`Topology::Lines`] — there is nothing else a
    /// second endpoint could mean, so it is inferred rather than declared
    /// twice. Assigning it on *some* paths is an error: a procedure either
    /// draws segments or it does not.
    ClipB,
    /// vertex: point sprite size in pixels. Under [`Topology::Lines`], the
    /// width of the stroke, uniform along the segment.
    PointSize,
    /// fragment: linear RGB, straight alpha.
    Color,
}

impl Output {
    pub const ALL: [Output; 4] = [Output::Clip, Output::ClipB, Output::PointSize, Output::Color];

    pub fn name(self) -> &'static str {
        match self {
            Output::Clip => "clip",
            Output::ClipB => "clip_b",
            Output::PointSize => "point_size",
            Output::Color => "color",
        }
    }

    pub fn from_name(s: &str) -> Option<Output> {
        Output::ALL.into_iter().find(|o| o.name() == s)
    }

    pub fn ty(self) -> Ty {
        match self {
            Output::Clip | Output::ClipB | Output::Color => Ty::Vec4,
            Output::PointSize => Ty::Float,
        }
    }

    pub fn block(self) -> BlockKind {
        match self {
            Output::Clip | Output::ClipB | Output::PointSize => BlockKind::Vertex,
            Output::Color => BlockKind::Fragment,
        }
    }
}

/// Values readable without declaration.
///
/// `Seed` is listed here because name resolution treats it like an ambient, but
/// it is a per-element carried attribute rather than a uniform: it is stored,
/// and compaction moves it with its element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ambient {
    Seed,
    Capacity,
    T,
    /// Musical position, in beats, on the session's tempo grid — the same
    /// instant [`Ambient::T`] names, counted in beats instead of seconds.
    ///
    /// Monotone while the tempo is positive and continuous across a tempo
    /// correction, which is what makes it safe to drive a phase from: a tempo
    /// change bends the rate and never moves a beat that has already happened.
    /// Fractional, so `fract(beats)` is the position within the beat and
    /// `beats * 0.25` counts bars in four.
    ///
    /// **Not a substitute for `t` and not a replacement for it.** `t` is the
    /// simulation's own clock and is what an accumulating procedure integrates
    /// against; `beats` is where the room is. A procedure that wants to move
    /// with the music reads this, and one that wants to move at a rate reads
    /// `t`.
    Beats,
    Dt,
    Camera,
    PointCoord,
    /// **Where the camera is**, in world space. L4 fragment only.
    ///
    /// One half of what a fullscreen procedure needs to march: a ray is an
    /// origin and a direction, and the origin is the same for every fragment.
    Eye,
    /// **The unit direction from [`Ambient::Eye`] through this fragment**, in
    /// world space. L4 fragment only.
    ///
    /// Given rather than derived, because the camera is a built-in here: a
    /// procedure that built its own would be restating the engine's projection
    /// convention, and getting it slightly wrong is a picture that looks nearly
    /// right. It also keeps a `mat4` inverse out of a language that has no
    /// operator for one.
    Ray,
}

impl Ambient {
    pub const ALL: [Ambient; 9] = [
        Ambient::Seed,
        Ambient::Capacity,
        Ambient::T,
        Ambient::Beats,
        Ambient::Dt,
        Ambient::Camera,
        Ambient::PointCoord,
        Ambient::Eye,
        Ambient::Ray,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Ambient::Seed => "seed",
            Ambient::Capacity => "capacity",
            Ambient::T => "t",
            Ambient::Beats => "beats",
            Ambient::Dt => "dt",
            Ambient::Camera => "camera",
            Ambient::PointCoord => "point_coord",
            Ambient::Eye => "eye",
            Ambient::Ray => "ray",
        }
    }

    pub fn from_name(s: &str) -> Option<Ambient> {
        Ambient::ALL.into_iter().find(|a| a.name() == s)
    }

    pub fn ty(self) -> Ty {
        match self {
            Ambient::Seed | Ambient::Capacity => Ty::Uint,
            Ambient::T | Ambient::Beats | Ambient::Dt => Ty::Float,
            Ambient::Camera => Ty::Mat4,
            Ambient::Eye | Ambient::Ray => Ty::Vec3,
            Ambient::PointCoord => Ty::Vec2,
        }
    }

    /// Blocks this value is readable in. `Seed` is readable everywhere.
    pub fn available_in(self, kind: Kind, block: BlockKind) -> bool {
        match self {
            Ambient::Seed | Ambient::T | Ambient::Beats => true,
            Ambient::Capacity | Ambient::Dt => kind == Kind::L1,
            Ambient::Camera => kind == Kind::L4,
            Ambient::PointCoord => block == BlockKind::Fragment,
            // Fragment-only, and not because a vertex stage could not be given
            // them: a fullscreen procedure has no vertex block at all, and in a
            // per-element one a ray through the fragment is not a thing a
            // vertex has. Offering them where they mean nothing would be one
            // more way to write a procedure that compiles and is wrong.
            Ambient::Eye | Ambient::Ray => {
                kind == Kind::L4 && block == BlockKind::Fragment
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lit {
    Float(f32),
    Int(i32),
    Uint(u32),
    Bool(bool),
}

impl Lit {
    pub fn ty(self) -> Ty {
        match self {
            Lit::Float(_) => Ty::Float,
            Lit::Int(_) => Ty::Int,
            Lit::Uint(_) => Ty::Uint,
            Lit::Bool(_) => Ty::Bool,
        }
    }
}

/// `param <name> : <type> [<min>, <max>] = <default>`
///
/// The range is mandatory: it is the fader range, the agent's search range, and
/// the normalisation basis for signal binding all at once. For vector params it
/// applies per component.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Ty,
    pub min: f32,
    pub max: f32,
    pub default: Expr,
    pub span: Span,
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

/// One `proc`, which is one file and fills one slot.
#[derive(Debug, Clone)]
pub struct Proc {
    pub name: String,
    pub kind: Kind,
    /// L1 only.
    pub topology: Option<Topology>,
    /// L1 only.
    pub capacity: Option<CapacityDecl>,
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
    /// L4: once per element.
    Vertex,
    /// L4: once per rasterised fragment.
    Fragment,
}

impl BlockKind {
    pub fn name(self) -> &'static str {
        match self {
            BlockKind::Spawn => "spawn",
            BlockKind::Element => "element",
            BlockKind::Vertex => "vertex",
            BlockKind::Fragment => "fragment",
        }
    }

    pub fn from_name(s: &str) -> Option<BlockKind> {
        Some(match s {
            "spawn" => BlockKind::Spawn,
            "element" => BlockKind::Element,
            "vertex" => BlockKind::Vertex,
            "fragment" => BlockKind::Fragment,
            _ => return None,
        })
    }

    pub fn kind(self) -> Kind {
        match self {
            BlockKind::Spawn | BlockKind::Element => Kind::L1,
            BlockKind::Vertex | BlockKind::Fragment => Kind::L4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let <name> = <expr>;` — immutable.
    Let {
        name: String,
        value: Expr,
        span: Span,
    },
    /// `var <name> = <expr>;` — mutable, block scoped, does not survive the frame.
    Var {
        name: String,
        value: Expr,
        span: Span,
    },
    /// `<target> = <expr>;` or `<target> op= <expr>;`
    ///
    /// The target is a bare name: whether it is a local, an attribute, or a
    /// stage output is decided during resolution. Compound assignment (`op` is
    /// `Some`) is legal on locals only — on an attribute it would look like
    /// accumulation while re-reading the previous frame every time.
    Assign {
        target: String,
        op: Option<BinOp>,
        value: Expr,
        span: Span,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        els: Vec<Stmt>,
        span: Span,
    },
    /// `for <var> in <start>..<end> { .. }` — bounds are compile-time constants,
    /// which is what makes cost estimation possible.
    For {
        var: String,
        start: i32,
        end: i32,
        body: Vec<Stmt>,
        span: Span,
    },
    /// `kill();` — L1 `element` only.
    Kill { span: Span },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. }
            | Stmt::Var { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::If { span, .. }
            | Stmt::For { span, .. }
            | Stmt::Kill { span } => *span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    /// `%`. Follows `mod` semantics on floats (always non-negative), which
    /// differs from WGSL; ordinary remainder on integers.
    Rem,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

impl BinOp {
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne
        )
    }

    pub fn is_logical(self) -> bool {
        matches!(self, BinOp::And | BinOp::Or)
    }

    /// Binding power, higher binds tighter. Comparison below arithmetic,
    /// logical below comparison — GLSL order, which is what LLMs will assume.
    pub fn precedence(self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::Eq | BinOp::Ne => 3,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 4,
            BinOp::Add | BinOp::Sub => 5,
            BinOp::Mul | BinOp::Div | BinOp::Rem => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit {
        value: Lit,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    Unary {
        op: UnOp,
        value: Box<Expr>,
        span: Span,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// Builtin calls (`hash1`, `curl`) and type constructors (`vec3`, `float`)
    /// are both this. The check pass tells them apart by name.
    Call {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// `v.xy`, `v.zyx`. `components` is the raw suffix, unvalidated.
    Swizzle {
        value: Box<Expr>,
        components: String,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Lit { span, .. }
            | Expr::Ident { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Call { span, .. }
            | Expr::Swizzle { span, .. } => *span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_names_round_trip() {
        for a in Attr::ALL {
            assert_eq!(Attr::from_name(a.name()), Some(a));
        }
    }

    #[test]
    fn id_is_not_an_attribute_or_an_ambient() {
        // Identity is `seed`. A slot index is not an identity once compaction
        // moves elements, so `id` must not resolve to anything.
        assert_eq!(Attr::from_name("id"), None);
        assert_eq!(Ambient::from_name("id"), None);
    }

    #[test]
    fn seed_is_readable_everywhere() {
        for (kind, block) in [
            (Kind::L1, BlockKind::Spawn),
            (Kind::L1, BlockKind::Element),
            (Kind::L4, BlockKind::Vertex),
            (Kind::L4, BlockKind::Fragment),
        ] {
            assert!(Ambient::Seed.available_in(kind, block));
        }
    }

    #[test]
    fn point_coord_is_fragment_only() {
        assert!(Ambient::PointCoord.available_in(Kind::L4, BlockKind::Fragment));
        assert!(!Ambient::PointCoord.available_in(Kind::L4, BlockKind::Vertex));
    }

    #[test]
    fn only_velocity_and_age_are_derivable() {
        let derivable: Vec<_> = Attr::ALL.into_iter().filter(|a| a.is_derivable()).collect();
        assert_eq!(derivable, vec![Attr::Velocity, Attr::Age]);
    }

    #[test]
    fn multiplication_binds_tighter_than_comparison() {
        assert!(BinOp::Mul.precedence() > BinOp::Lt.precedence());
        assert!(BinOp::Add.precedence() > BinOp::Eq.precedence());
    }
}
