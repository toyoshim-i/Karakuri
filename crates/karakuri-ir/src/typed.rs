//! The resolved IR: what the check pass produces and code generation consumes.
//!
//! This is a second tree rather than annotations hung off the first one,
//! because every consumer needs a type on every node and none of them should
//! have to ask a side table for it. WGSL has no implicit conversions, so the
//! generator emits what the tree says and never infers; if a type is missing
//! at that point it is too late to work it out.
//!
//! Three things are resolved here that the parser deliberately left open:
//!
//! - **Names.** A bare identifier becomes a local, a param, an attribute, or
//!   an ambient. The parser could not know which — a procedure may declare
//!   `param energy`, so whether `energy` resolves depends on declarations.
//! - **Calls.** `Expr::Call` splits into a builtin with its overload chosen and
//!   a type constructor.
//! - **Compound assignment.** `x += e` is desugared to `x = x + e` here, so
//!   code generation has one case instead of two and the "compound assignment
//!   is for locals only" rule is enforced in exactly one place.

use crate::ast::{Ambient, Attr, BinOp, Blend, BlockKind, CapacityDecl, Kind, Lit, Output, Param, Topology, Ty};
use crate::builtin::Builtin;
use crate::span::Span;

/// A procedure that has passed parsing, type checking, and contract checking.
///
/// Holding one of these is the claim that the rules in `docs/ir-spec.md` hold
/// of it: `consumes` is contained in `emit`, nothing reads the signal bus,
/// and every emitted attribute and required stage output is assigned on
/// every path. There is no derivation step: an unmet `consumes` is a plain
/// rejection, whether caught here (a single procedure declaring both `emit`
/// and `consumes` — see `check_consumes_emitted` in `check.rs`) or, for the
/// ordinary L1/L4 pairing, at Set-composition time outside this crate.
#[derive(Debug, Clone)]
pub struct Checked {
    pub name: String,
    pub kind: Kind,
    /// **What the geometry is on an L1, and what the renderer draws on an L4.**
    ///
    /// Declared by an L1 file; *inferred* for an L4 from whether its `vertex`
    /// block assigns [`Output::ClipB`], because a second endpoint is the only
    /// thing that could make a procedure draw segments and a header field
    /// would be a second place for the same fact to be wrong. `None` only
    /// where checking already failed.
    ///
    /// One field for both because the two are the same question asked of the
    /// two halves of a Set. Before `lines` existed this was an L1 field with
    /// no L4 counterpart, which is what made `docs/roadmap.md`'s "give an L4 a
    /// way to say what it renders" an open question.
    ///
    /// **The two halves are allowed to disagree**, and nothing compares them:
    /// only the L4's value reaches lowering, and a line renderer needs nothing
    /// of the geometry that Set composition does not already check. That is
    /// what lets one L1 be drawn as sprites by one L4 and as strokes by
    /// another. See the comment in `Set::build`.
    pub topology: Option<Topology>,
    pub capacity: Option<CapacityDecl>,
    /// **How many output elements this L2 makes per input element**, from
    /// `amplify`. `None` is the endomorphism every L2 was before amplification
    /// existed, and is *not* the same statement as `Some(1)`: nothing else in
    /// the language distinguishes them, but the lowering does — a node with no
    /// declaration generates the shader it always generated, shares its input's
    /// liveness and its input's counts, and allocates nothing.
    pub amplify: Option<u32>,
    pub blend: Option<Blend>,
    pub params: Vec<Param>,
    pub emit: Vec<Attr>,
    pub consumes: Vec<Attr>,
    pub blocks: Vec<TBlock>,
    /// Filled by cost estimation. `None` until stage 4 has run.
    pub cost: Option<Cost>,
    /// **Whether this procedure can be evaluated at any `t` directly** —
    /// `docs/ir-spec.md`, "Closed form versus accumulating".
    ///
    /// True means the procedure's state at time `t` is a pure function of
    /// `seed`, `t`, and its parameters. It buys two things and the second is
    /// the larger one:
    ///
    /// - **No priming.** Cold to Live with no warm-up, because there is no
    ///   accumulated state to warm.
    /// - **It can be scrubbed.** Forward at any rate, held, or *backwards* —
    ///   tape-style transport. An accumulating procedure can only go forward
    ///   one step at a time, and reversing it is not slow but impossible:
    ///   there is no un-integrating a sum.
    ///
    /// **Necessary for a seek, not sufficient for one.** Priming only ever runs
    /// forward from a state the engine already has, so the procedure is all it
    /// needs. Seeking to an arbitrary `t` also needs everything *else* that is
    /// a function of time at that instant to be evaluable there, and
    /// [`Ambient::Beats`](crate::ast::Ambient::Beats) has made that concrete:
    /// a procedure reading it is a function of the tempo grid as well as of
    /// `t`, and the grid at a past `t` depends on the correction history rather
    /// than on `t`.
    ///
    /// It is still not a disqualifier. The grid can be evaluated anywhere **as
    /// it stands**, which is a pure function of `t` given the current tempo and
    /// anchor — and for a scrub that is the wanted answer, since seeking to bar
    /// 32 means bar 32 of the grid the room is on now. For an exact re-run of a
    /// past moment it is not, and nothing keeps the history that would be.
    /// Whatever builds the transport owes that half and owes this distinction
    /// with it; this flag covers neither.
    ///
    /// Decided by [`check`](crate::check::check) and **deliberately
    /// conservative**: see `is_closed_form` there for exactly what it refuses
    /// to claim and why under-claiming is the safe direction.
    ///
    /// Vacuously true for an L4 procedure, which emits nothing and holds no
    /// per-element state at all. A *Set* is closed form when both of its
    /// procedures are, which in practice means when its L1 is.
    pub closed_form: bool,
    /// **Whether the procedure reads [`Ambient::Beats`](crate::ast::Ambient::Beats).**
    ///
    /// Material written against the grid already follows the room's tempo. A
    /// transport that also scales the slot's clock by the tempo would make it
    /// follow twice — the clock scaled, and the grid read on the scaled clock —
    /// so the two compound into roughly the square of the tempo ratio. This is
    /// what lets a surface refuse that combination instead of offering it.
    ///
    /// A fact rather than a judgement: nothing is rejected for it, and unlike
    /// [`Checked::closed_form`] there is no safe direction to err in. Claiming
    /// it wrongly greys out a control that would have worked; missing it offers
    /// one that compounds.
    ///
    /// Says nothing about *beat* sync, which is a position lock and is decided
    /// by `closed_form` instead. The two controls are independent and this
    /// bears on only one of them.
    pub reads_beats: bool,
    pub span: Span,
}

impl Checked {
    pub fn block(&self, kind: BlockKind) -> Option<&TBlock> {
        self.blocks.iter().find(|b| b.kind == kind)
    }

    /// Whether every `capacity` element is live from frame zero. A procedure
    /// with no `spawn` block never allocates, so `seed` is the initial slot
    /// index and lattice generators work as written.
    pub fn is_static(&self) -> bool {
        self.kind == Kind::L1 && self.block(BlockKind::Spawn).is_none()
    }
}

/// What an artifact records about its own expense.
///
/// Per element, never total: `capacity` belongs to the Set, so an artifact has
/// no total cost to be judged on. The cost at any capacity is a multiplication,
/// and that per-element figure is the intrinsic property of the procedure.
///
/// The op counts are **separate quantities and must not be summed**, a thing
/// that is easy to get wrong because they share a unit. Each scales with
/// something different: `element` with live population, `spawn` with spawn rate,
/// `fragment` with covered pixels, and `evaluation` with how often a caller
/// evaluates a field. Adding them charges a one-off spawn cost on every frame
/// for the life of the element, and charges fill rate as though it were
/// geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cost {
    /// Per live element, per frame: the L1 `element` block or the L4 `vertex`
    /// block. This is the figure the ceiling applies to and the one the
    /// artifact publishes.
    pub ops_per_element: u64,
    /// Once per element, when it is allocated. Zero without a `spawn` block.
    /// Amortised over the element's whole life, so it earns a looser ceiling
    /// than the per-frame figure rather than a share of it.
    pub ops_per_spawn: u64,
    /// Per rasterised fragment, not per element. Zero for L1. Point sprite cost
    /// is fill-rate bound — `point_size` and resolution decide the overdraw —
    /// which is exactly why an L4 artifact publishes a measurement at reference
    /// conditions instead of a per-element number.
    pub ops_per_fragment: u64,
    /// **Per evaluation of a `kind Field` procedure**, and zero for every other
    /// kind. A fourth quantity rather than a share of the three above, because
    /// what it scales with is *how often the caller calls it* — once per element
    /// in a `vertex`, forty-eight times in a march loop — which is a property of
    /// the caller and not of the field.
    ///
    /// It carries no ceiling of its own for the same reason: a field is not
    /// expensive or cheap on its own terms, and the number that has to fit under
    /// a ceiling is the caller's, with this multiplied into it. That
    /// multiplication happens where the Set is built, which is the first point
    /// holding both procedures.
    pub ops_per_evaluation: u64,
    /// Bytes of attribute storage per element, both buffers counted.
    pub bytes_per_element: u32,
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
    /// ambient, and not another local — so a name identifies one binding
    /// within a block and code generation can emit it verbatim.
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
    /// Writes the next frame's buffer. Reading the same attribute anywhere in
    /// the block still yields the previous frame's value.
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
    /// A read, which is always of the previous frame's value.
    Attr(Attr),
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
    /// `vec3(a, b, c)`, `vec3(x)` broadcasting, and the scalar conversions
    /// `float(i)` / `int(x)` / `uint(x)`. All of them construct `ty` from
    /// `args`; there are no implicit conversions anywhere else, so a cast in
    /// the generated WGSL corresponds to one of these and nothing else.
    Construct {
        args: Vec<TExpr>,
    },
    /// `v.xy`, `v.zyx`. Components are 0-based indices, already validated
    /// against the source width.
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
