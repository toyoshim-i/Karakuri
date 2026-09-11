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

use crate::ast::{
    Ambient, Attr, BinOp, Blend, BlockKind, CapacityDecl, Kind, Lit, Output, Param, SlotTy,
    Topology, Ty,
};
use crate::builtin::Builtin;
use crate::span::Span;

/// **The name a procedure's header gives one declared input** — `far` in
/// `uses far : Geometry`. It is what an `edge` is written against and what a
/// Set's `--edge <node>.<slot>=<node>` names on the near side of the dot.
///
/// **Its own type because `slot` names three unrelated things in this
/// workspace**: a layer's position inside a Set, a Set's position in the
/// deck, and this — the name a procedure reads a binding through, resolved
/// nowhere else. See
/// `docs/adr/0344-slot-is-disambiguated-into-three-types-and-adr-0049s-wait-is-over.md`.
///
/// **`karakuri_store::record::InputPort` and `karakuri_operation::InputPort`
/// are the same concept, mirrored rather than shared.** Neither
/// `karakuri-store` nor `karakuri-operation` depends on this crate —
/// `karakuri-operation` is a leaf by charter and a serialised record wants its
/// own `Serialize`/`Deserialize` — so each defines its own copy, on
/// `karakuri_operation::NodeAddress`'s precedent: three definitions agreeing costs
/// less than a dependency none of the three crates already pays for.
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

/// **One slot a procedure declares**, checked — the name it is read through
/// and the type it takes.
///
/// The pair is what a binding needs both halves of: the name is what an `edge`
/// names, and the type is what decides whether the thing bound to it is the
/// right kind of node. Kept together so that neither can be resolved against
/// the wrong one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub name: InputPort,
    pub ty: SlotTy,
}

/// **Which texture a fetch reads**, resolved from the name at the call site.
///
/// Three of them and no more: an L5 is handed the incoming picture, may be
/// handed a retained cut of the previous frame where it declares `retains`, and
/// may declare any number of further inputs as `uses … : Texture` slots. What
/// fills each is decided outside the file — the chain slot's position, the
/// slot's `cut`, the Set's `edge`s — which is why the file names them and never
/// says where they come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TexRef {
    /// `src` — the incoming texture, implicit and undeclared.
    Src,
    /// `held` — the retained cut, readable only under `retains`.
    Held,
    /// A `uses <name> : Texture` slot, under the name the header gave it. The
    /// name is carried for the reason [`TExprKind::Field`]'s is: it is what the
    /// lowering addresses the binding under, and what an `edge` is written
    /// against.
    Slot(InputPort),
}

/// A procedure that has passed parsing, type checking, and contract checking.
///
/// Holding one of these is the claim that the rules in `docs/ir-spec.md` hold
/// of it: `consumes` is contained in `emit`, nothing reads the signal bus,
/// and every emitted attribute and required stage output is assigned on
/// every path. What it is *not* is the claim that everything consumed is
/// emitted: `age` and `velocity` are derived, and whether anybody in a chain
/// emits a given attribute is a question no single procedure can answer. Both
/// halves settle at Set-composition time, outside this crate; what settles
/// here is the part one file can answer — see `check_consumes_emitted` in
/// `check.rs`.
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
    /// no L4 counterpart, which is what left "give an L4 a way to say what it
    /// renders" an open question.
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
    /// **The slots this procedure declares** — see [`crate::ast::UsesDecl`].
    /// `uses far : Geometry` on the header is one [`Slot`], and the far
    /// element's attributes are then read as `far.<attr>`.
    ///
    /// The name is carried rather than reduced to a flag because it is what a
    /// Set's `edge` is written against: the file says what it needs and the Set
    /// says what fills it, and neither half can be resolved without the other's
    /// spelling. The correspondence between the two geometries is the slot
    /// index, which is only the same element in both while neither compacts.
    ///
    /// **A list, though the checker admits at most one today.** What a reader
    /// wants of it is "the geometry slot", and that is a question about a type
    /// rather than about a position — [`Checked::geometry_slot`] asks it. The
    /// arity is the check pass's rule and stays there, where the refusal that
    /// enforces it is written; a shape that could only hold one would put a
    /// second copy of that rule in the type.
    pub uses: Vec<Slot>,
    /// **Whether this procedure reads a retained cut of the previous frame**,
    /// from a bare `retains` on the header. L5's, and false everywhere else.
    ///
    /// A `bool` here where the AST carries a span, because what a consumer
    /// wants is the fact: the engine allocates a retention only where a slot's
    /// answer names one, and the lowering declares a second texture binding.
    /// *Which* cut is not in this type and must not be — the file does not know
    /// it.
    pub retains: bool,
    pub blend: Option<Blend>,
    pub params: Vec<Param>,
    pub emit: Vec<Attr>,
    pub consumes: Vec<Attr>,
    pub blocks: Vec<TBlock>,
    /// **Always `None`, and that is not what this field was for.**
    ///
    /// It was meant to be filled by stage 4, and nothing fills it:
    /// `cost::estimate` takes a `&Checked` and returns its answer, and every
    /// caller uses the return value. Two checks were written against this field
    /// and were silently dead until a test asked one of them a question it
    /// could not answer.
    ///
    /// Kept rather than deleted because the shape is still the right one — a
    /// checked procedure ought to carry what it costs — but until something
    /// fills it, **ask `cost::estimate` instead of reading this**.
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

    /// **The name of the geometry slot this procedure declares**, and `None`
    /// for one that declares none.
    ///
    /// Asked by name rather than by position, because every caller wants the
    /// geometry one specifically: what a lowering binds is a buffer of
    /// elements, and what a Set's edge resolves against is a node with
    /// elements in it. A slot of another type answers a different question and
    /// must not answer this one by being first in the list.
    pub fn geometry_slot(&self) -> Option<&str> {
        self.uses
            .iter()
            .find(|s| s.ty == SlotTy::Geometry)
            .map(|s| s.name.as_str())
    }

    /// **The fields this procedure declares**, in the order the header wrote
    /// them, and empty for one that declares none.
    ///
    /// A list where [`Checked::geometry_slot`] is an option, and the difference
    /// is the whole of what the type on a slot is for: there is at most one
    /// second element buffer per node, and there is no such limit on fields —
    /// a marcher wanting a shape and a cutter is the ordinary case, and a field
    /// has no node for a second one to need.
    ///
    /// Every caller wants the Field ones specifically: a lowering splices a
    /// body per name here, and a Set's edge resolves each against a `kind
    /// Field` procedure. A slot of another type answers a different question
    /// and must not answer this one by being in the list.
    pub fn field_slots(&self) -> Vec<&str> {
        self.uses
            .iter()
            .filter(|s| s.ty == SlotTy::Field)
            .map(|s| s.name.as_str())
            .collect()
    }

    /// **The name of the camera slot this procedure declares**, and `None` for
    /// one that declares none — which draws with the Set's camera, as every
    /// renderer written before this notation did.
    ///
    /// An option where [`Checked::field_slots`] is a list, on
    /// [`Checked::geometry_slot`]'s terms: a renderer draws one picture and a
    /// picture is seen from one place, so a second camera slot is refused by
    /// the check pass rather than shaped away here.
    pub fn camera_slot(&self) -> Option<&str> {
        self.uses
            .iter()
            .find(|s| s.ty == SlotTy::Camera)
            .map(|s| s.name.as_str())
    }

    /// **The sources this procedure names**, in header order, and empty for one
    /// that names none.
    ///
    /// A list on [`Checked::field_slots`]'s terms rather than an option on the
    /// two above: what a Source slot costs is one `u32` in a uniform block the
    /// module already has, so there is nothing for an arity rule to protect —
    /// and `source == a || source == b` is an ordinary thing for a mask to
    /// want. Every caller wants the Source ones specifically: what fills one is
    /// an L1's assigned identity, and a slot of another type answers a
    /// different question.
    /// **The pictures this procedure folds in**, in header order, and empty for
    /// a chain slot's L5 — which declares none, because the chain's only fan-in
    /// is its order.
    ///
    /// A list on [`Checked::field_slots`]'s terms: a fold of three is as
    /// ordinary as a fold of two, and nothing caps it but what a bind group can
    /// hold.
    pub fn texture_slots(&self) -> Vec<&str> {
        self.uses
            .iter()
            .filter(|s| s.ty == SlotTy::Texture)
            .map(|s| s.name.as_str())
            .collect()
    }

    pub fn source_slots(&self) -> Vec<&str> {
        self.uses
            .iter()
            .filter(|s| s.ty == SlotTy::Source)
            .map(|s| s.name.as_str())
            .collect()
    }

    /// **Whether nothing ever moves an element between slots**, so that `seed`
    /// is the slot index for the whole run.
    ///
    /// Two things move elements and this asks about both. A `spawn` block
    /// allocates, and `kill()` makes the next step's scan compact the survivors
    /// down — and compaction is order preserving, which keeps blend order
    /// stable and moves every element after the gap.
    ///
    /// **It used to ask only about `spawn`**, and its own sentence — "every
    /// `capacity` element is live from frame zero" — was false for a procedure
    /// that kills without spawning: those are all live at frame zero and are not
    /// after it. Nothing called it, so nothing was wrong yet; the predicate a
    /// caller will want is this one, and it is the one `karakuri-codegen` has
    /// been computing all along to decide whether to generate the compacted
    /// element entry.
    pub fn is_static(&self) -> bool {
        self.kind == Kind::L1
            && self.block(BlockKind::Spawn).is_none()
            && !self.blocks.iter().any(|b| contains_kill(&b.stmts))
    }
}

/// Whether these statements can remove an element.
///
/// In this crate rather than in the generator that first needed it, because two
/// callers want it now and they are in different crates: the lowering, to choose
/// the compacted `element` entry point, and [`Checked::is_static`], to say
/// whether `seed` is the slot index.
pub fn contains_kill(stmts: &[TStmt]) -> bool {
    stmts.iter().any(|s| match s {
        TStmt::Kill { .. } => true,
        TStmt::If { then, els, .. } => contains_kill(then) || contains_kill(els),
        TStmt::For { body, .. } => contains_kill(body),
        TStmt::Let { .. } | TStmt::Var { .. } | TStmt::Assign { .. } => false,
    })
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
/// Field evaluations by **one slot**, per unit of each of [`Cost`]'s
/// quantities.
///
/// Three counters rather than one, because which ceiling an evaluation charges
/// is decided by the block it sits in: one in a `vertex` happens per element
/// and one in a `fragment` per covered pixel, and adding them would charge fill
/// rate as though it were geometry — the same mistake the three op counts are
/// separate to prevent.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SlotCalls {
    /// What the procedure's header called the field these calls reach.
    pub slot: String,
    pub per_element: u64,
    pub per_spawn: u64,
    pub per_fragment: u64,
    /// **Every call, in every block, including the ones no ceiling is charged
    /// for.** A `camera` block scales with nothing and is charged to none of the
    /// three above, so its calls landed nowhere and [`FieldCalls::any`] said an
    /// L3 evaluating a field did not — which made the Set's refusal skip it and
    /// the process die on a shader naming a function nothing had spliced in.
    ///
    /// Counted separately rather than folded into one of the three, because
    /// the three are *rates* and this is not: what it answers is "does this
    /// procedure evaluate this field", which has no denominator.
    pub total: u64,
}

/// How often a procedure evaluates each of the fields it declares.
///
/// **Labelled, where it used to be one anonymous set of counters.** A count
/// with no slot on it was answerable while `field(p)` named the one field by
/// being the only spelling there was; a procedure that takes a shape and a
/// cutter multiplies two different `ops_per_evaluation` figures, and a sum
/// across both cannot be attributed to either. So the ceiling arithmetic adds
/// the slots up and the refusal names the one that dominates.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldCalls {
    /// One entry per slot the procedure actually calls, in first-call order.
    /// A declared slot nothing evaluates has no entry: what this counts is
    /// calls, and the header is where the declaration is recorded.
    pub slots: Vec<SlotCalls>,
}

impl FieldCalls {
    /// Whether this procedure evaluates any field at all — which is what
    /// decides whether its module needs one spliced into it.
    ///
    /// **Asks the totals, not the three rates.** A block charged to no ceiling
    /// still calls the function.
    pub fn any(&self) -> bool {
        self.slots.iter().any(|s| s.total > 0)
    }

    /// The counters for one slot, and `None` for a slot this procedure declares
    /// and never calls.
    pub fn slot(&self, slot: &str) -> Option<&SlotCalls> {
        self.slots.iter().find(|s| s.slot == slot)
    }

    /// The counters for `slot`, created empty if this is its first call.
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

/// **No longer `Copy`**, because [`FieldCalls`] carries the slot names — and a
/// name is what makes a refusal about several fields say which one it is about.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
    /// is fill-rate bound — `point_rate` and resolution decide the overdraw —
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
    /// **How many times this procedure evaluates the Set's field**, per unit of
    /// each quantity above.
    ///
    /// Not a cost: a count, which the Set multiplies by the field's
    /// `ops_per_evaluation` and adds to the figure on the matching axis. It is
    /// here rather than in the engine because it is what cost estimation
    /// already knows and nothing else does — a `shape(p)` inside `for i in
    /// 0..48` is forty-eight evaluations of `shape`, and only the pass that
    /// resolves loop bounds can say so.
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
    /// A read of the **far** element's attribute — the geometry bound to this
    /// node's declared slot, at the same element index. Written `<slot>.<name>`,
    /// where the slot is whatever the procedure's `uses` called it.
    ///
    /// The name is not carried here, and that is not an omission: a node takes
    /// one second geometry, so the read has nowhere else to point. What the
    /// name decides is which spellings *reach* this variant, which the checker
    /// has already settled by the time one exists.
    ///
    /// Its own variant rather than a flag on [`TExprKind::Attr`], because every
    /// pass that walks attribute reads has to decide about it: cost weighs it
    /// the same, the lowering addresses a different buffer, and
    /// `is_closed_form` treats it as a read of carried state exactly as it
    /// treats the near side.
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
    /// **The distance a field gives at a point** — `<slot>(p)`, where the slot
    /// is whatever this procedure's `uses` called it.
    ///
    /// Its own variant rather than a builtin, because there is no body for it
    /// here: it lowers to a call of the function another file was spliced in
    /// as, so a Set that binds the slot to nothing cannot satisfy it. That was
    /// true of `Builtin::Field` too, and the difference is the name — **which
    /// is carried, unlike [`TExprKind::Far`]'s.** A geometry read has nowhere
    /// else to point, because a node takes one second geometry; a field call
    /// does, because a procedure may declare several, and the slot is what the
    /// lowering addresses the function and the params under.
    Field {
        slot: String,
        point: Box<TExpr>,
    },
    /// **The identity of the geometry bound to a declared Source slot** —
    /// `only`, alone, where the header said `uses only : Source`.
    ///
    /// **A value rather than a member or a call**, which is what separates it
    /// from the three slot reads beside it: a geometry has no type, a field has
    /// no value until it is evaluated somewhere, and a camera is six numbers —
    /// this is a `uint`, and the language has one of those.
    ///
    /// Its own variant rather than an [`Ambient`], because the answer is per
    /// *slot*: `Ambient::Source` is the instance this chain runs over and there
    /// is one of it, while a procedure may declare several of these and each
    /// resolves to whichever L1 its own edge named. The name is carried for the
    /// reason [`TExprKind::Field`]'s is — it is what the lowering addresses the
    /// uniform field under, and what the engine writes the salt into.
    Source {
        slot: String,
    },
    /// **A fetch from a texture an L5 was handed** — `texel(src)`,
    /// `tap(held, uv)`, `tap(<slot>, uv)`.
    ///
    /// **Its own variant rather than a [`TExprKind::Builtin`] with a texture
    /// argument**, and the reason is that there is no such argument: a texture
    /// is not a value this language has a [`Ty`] for, so what the call site
    /// names is a *binding* and the tree carries which one rather than an
    /// expression that evaluates to it. Making it a `Ty` would give `let x =
    /// src;` a type and put the refusal two lines later than the mistake.
    ///
    /// `func` is [`Builtin::Texel`] or [`Builtin::Tap`], kept so that the one
    /// table still says what each costs and what each is called — the check
    /// pass, cost estimation and the lowering read it here exactly as they read
    /// it for every other builtin.
    Sample {
        func: Builtin,
        texture: TexRef,
        /// The frame coordinate, for `tap`, and `None` for `texel` — which
        /// takes no coordinate on purpose. See [`TexRef`].
        at: Option<Box<TExpr>>,
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
