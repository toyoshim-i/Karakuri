use crate::span::Span;

use super::*;

/// `param <name> : <type> [<min>, <max>] = <default>`
///
/// The range is mandatory: it is the fader range, the agent's search range, and
/// the normalisation basis for signal binding all at once. For vector params it
/// applies per component — one `[min, max]` covers `glow.x`, `glow.y` and
/// `glow.z` alike, which is `docs/ir-spec.md`'s "param" section and is what
/// `Set::build` writes into the range map under each of [`Param::keys`].
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
    /// The declared default as a number, or `None` where the declaration is an
    /// expression this does not fold.
    ///
    /// A negation is folded, because the parser does not fold it. `= -0.35` is
    /// `Unary { Neg, Lit }` and not a literal, so matching [`Expr::Lit`] alone
    /// silently dropped every negative default: the engine's param never entered
    /// its uniform, its declared value was discarded, binding refused it, and the
    /// shader got whatever the miss produced — a panic on the render thread before
    /// that reader returned an `Option`, and a quiet `0.0` after. A `.kir`
    /// declaring `param drift : float [-1.0, 1.0] = -0.35` is legal and none of
    /// that is a reader's to decide.
    ///
    /// Not general constant folding, deliberately. A default is checked in an empty
    /// scope, so it is *some* constant, but the useful set is one literal with an
    /// optional sign in front of it. Widening it is a language question — what a
    /// default may say — rather than a convenience for one caller, and it belongs
    /// here where every caller gets the same answer.
    ///
    /// It lives here rather than in a reader, and that is the whole point. It was
    /// private to `karakuri-engine`'s `Set`, with a note saying a second evaluator
    /// elsewhere would agree with the shader only by coincidence. There is now a
    /// second reader — `karakuri-environment`'s metadata writer records this number
    /// in a `param_decl` — and a metadata file whose `default` disagreed with the
    /// uniform the run actually loaded would be a card describing a procedure
    /// nobody ran. One function, so they cannot differ.
    ///
    /// `None` is *"this default is not a number I can state"*, and never *"there is
    /// no default"*: the grammar makes `= <expr>` mandatory. What a caller does
    /// with that is the caller's — the engine leaves the param out of its value
    /// map, so its uniform field is packed with the `0.0` a miss produces; the
    /// metadata writer writes the declaration with no `default` key.
    ///
    /// A vector declaration answers `None` here and is not undeclarable. `=
    /// vec3(0.4, 0.7, 1.0)` is three numbers and this returns one, so it is
    /// [`Param::default_components`] that states them — the widening this paragraph
    /// reserved, taken for the one case where the numbers are statable and the
    /// width was the whole obstacle. A caller that wants *one* number still wants
    /// this one.
    pub fn default_scalar(&self) -> Option<f32> {
        fold_literal(&self.default)
    }

    /// The declared default as one number per component, or `None` where the
    /// declaration is an expression this does not fold.
    ///
    /// [`Param::default_scalar`] widened by exactly one step, and it is the step
    /// that widening was always reserved for: that function's own documentation
    /// says `None` means *"this default is not a number I can state"* and never
    /// *"there is no default"*, and for a `vec3` the numbers are statable — the
    /// language just needs more than one of them to state them in. A parameter is
    /// driven one component at a time
    /// ([ADR-0268](../../../docs/adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)),
    /// so this is the shape every consumer of a default wants.
    ///
    /// What folds:
    ///
    /// - a float literal, or a negated one — one value, which is
    ///   [`Param::default_scalar`]'s answer in a one-element vector;
    /// - `vecN(a, b, …)` with `N` arguments, each a literal or a negated literal —
    ///   `N` values, in the order they are written;
    /// - `vecN(a)` with one such argument — the broadcast, `N` copies of it.
    ///
    /// `docs/ir-spec.md`, "Types": *"Vector constructors follow GLSL: any mix of scalars
    /// and shorter vectors whose component counts sum to the target width, or a single scalar
    /// to broadcast. `vec3(1.0, 0.0, 0.0)`, `vec3(0.0)`, and `vec4(position, 1.0)` are all
    /// well formed"*.
    ///
    /// A nested constructor answers `None`, and it is said here rather than left to
    /// be discovered. `vec3(vec2(0.1, 0.2), 0.3)` is legal by that same passage and
    /// its component count reaches the target width through an inner constructor
    /// rather than through this argument list, so the arity test above rejects it.
    /// That is [`Param::default_scalar`]'s deliberate narrowness held to: the
    /// useful set is literals with an optional sign, and widening it further is a
    /// language question — what a default may say — rather than a convenience for
    /// one caller.
    ///
    /// The arity is the declared type's and not the constructor's, so a list that
    /// does not fill the declaration folds to nothing rather than to a short
    /// vector. The checker already refuses such a default (`check.rs`, *"param `{}`
    /// default has type `{}`, expected `{}`"*); this answers for a `Param` that has
    /// not been through it.
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

    /// Every key this declaration is driven by, in `x`, `y`, `z` order.
    ///
    /// One key for a `float` — the declared name itself, so nothing about a scalar
    /// parameter changes — and one per component for a `vec2` or a `vec3`, spelled
    /// by [`component_key`].
    ///
    /// The order is load-bearing: `Set::published` walks this to build the default
    /// interface, and a control's *position* in that interface is what a MIDI
    /// control is learned against.
    pub fn keys(&self) -> Vec<String> {
        match self.ty.param_components() {
            Some(width) if width > 1 => (0..width).map(|i| component_key(&self.name, i)).collect(),
            _ => vec![self.name.clone()],
        }
    }
}

/// A literal, or a negated literal, as a number.
///
/// One fold, because two would disagree. [`Param::default_scalar`] and
/// [`Param::default_components`] are the same reading of the same declaration
/// at two widths, and the engine's uniform and `karakuri-environment`'s
/// `param_decl` are both packed from it — a second evaluator would agree with
/// the shader only by coincidence.
///
/// A negation is folded, because the parser does not fold it. `= -0.35` is
/// `Unary { Neg, Lit }` and not a literal, so matching [`Expr::Lit`] alone
/// silently dropped every negative default.
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

/// The component letters, in the order a vector parameter is addressed in.
///
/// Three, because [`Ty::param_components`] answers for the three types a
/// `param` may declare and the widest is a `vec3` — `check.rs` refuses the
/// rest: *"params may only be `float`, `vec2`, or `vec3`"*.
///
/// They are the language's own swizzle components — `check_swizzle` maps
/// exactly `x`, `y`, `z`, `w` — so a component key reads as the `.kir` text
/// that would name the same number.
pub const COMPONENTS: [&str; 3] = ["x", "y", "z"];

/// The key one component of a vector parameter is addressed by: `glow`
/// component 1 is `glow.y`.
///
/// `.` is the separator for three reasons, and each is checkable. No `.kir`
/// identifier can contain one — `lexer.rs`'s `lex_ident` takes
/// `is_alphanumeric() || c == '_'` and nothing else — so a component key
/// collides with no declared name however it is spelled. It is the language's
/// own spelling for *this component of that vector*, which is the swizzle
/// (`docs/ir-spec.md`, "Types": *"Swizzles allowed (`v.xy`, `v.zyx`)"*). And
/// the command line already spells *member of* this way: `--edge
/// <node>.<slot>=<node>`, whose parser splits on the last `.` for the same
/// reason this needs no escaping — *"the slot is a `.kir` identifier, which
/// cannot"* hold one.
///
/// Nothing in `karakuri-codegen` produces or consumes a `.`: the four manglings
/// are `param_{name}`, `field_{slot}_{name}`, `source_{slot}` and `usr_{name}`,
/// and the two *semantic* keys separate with `\u{1}`. So a component key never
/// reaches a uniform layout field name, which is what keeps
/// `node::write_params` walking the declaration names while the interface walks
/// these.
///
/// Panics on a component past `COMPONENTS`, which is unreachable through
/// [`Param::keys`]: the widest `param` a procedure may declare is a `vec3`.
pub fn component_key(name: &str, component: usize) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    push_component_key(&mut out, name, component);
    out
}

/// [`component_key`] into a buffer the caller owns.
///
/// This exists for the render thread. `karakuri-engine`'s `node::write_params`
/// composes a key per component of every vector param of every node, every
/// frame, and `format!` there would be an allocation per component per frame.
/// One reused buffer costs none after the first.
///
/// It is the same function so that the separator is written down once: two
/// spellings of `.` in two crates is exactly the drift `docs/contributing.md`
/// §4 is about.
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

/// Elements per geometry where nothing else names one — neither `--capacity`
/// nor the procedure's own [`CapacityDecl`].
///
/// A default of the language rather than of a flag, which is why it sits beside
/// the grammar it completes rather than in whichever surface last needed a
/// number. The order it is last in: `--capacity` overrides every source; below
/// it a procedure runs at the default its own `capacity` declares; this is what
/// is left when neither spoke.
///
/// Little should ever reach it. `check_header` requires a `capacity` on every
/// L1, so a [`Checked`](crate::typed::Checked) that passed contract checking
/// always carries one and the arm this fills is the one that says so:
/// `capacity` is an `Option` on the seam type, and a caller holding a procedure
/// that failed checking still needs a number rather than a panic.
pub const DEFAULT_CAPACITY: u32 = 262_144;

/// `amplify <factor>`
///
/// A compile-time constant, on the same terms as a loop bound, because the
/// output buffer is sized from it and the cost is multiplied out by it —
/// neither of which a runtime value could do. It is the one thing in the
/// language that changes an element count, which is why it is a header
/// declaration rather than anything a block can say: what a `deform` writes is
/// decided before it runs.
#[derive(Debug, Clone, Copy)]
pub struct AmplifyDecl {
    pub factor: u32,
    pub span: Span,
}

/// `uses <name> : Geometry`, `: Field`, `: Camera`, `: Source`
///
/// One input this node takes, named by the procedure and bound by the Set.
///
/// The name is the *procedure's own*, exactly as `consumes position` names an
/// attribute without naming which L1 supplies it. That is what keeps a `.kir` a
/// library part: a file that named a node would be coupled to one Set and could
/// not be used in another. What fills the slot is written where the use is
/// recorded — an `edge` in the Set file, `--edge <node>.<slot>=<node>` on the
/// command line — and an unbound slot is refused rather than filled in from
/// whatever happened to be lying around.
///
/// The type decides every rule about it, which is why it is written and carried
/// rather than checked and dropped — see [`SlotTy`]. Which kinds may declare
/// one, how many are legal, what an `edge` may bind it to and how it is read
/// all differ between the four, and each of those refusals is a sentence about
/// a type rather than about `uses`.
#[derive(Debug, Clone)]
pub struct UsesDecl {
    pub name: String,
    /// The name alone, so a refusal about what it collides with points at it rather
    /// than at the whole declaration.
    pub name_span: Span,
    /// What the slot takes, as the header spelled it.
    pub ty: SlotTy,
    pub span: Span,
}

/// `retains` — a bare declaration and no operand.
///
/// It says *this procedure reads a retained frame*, and that is the whole of
/// what it says: it makes [`TEXTURE_HELD`] readable in the `frame` block and
/// decides nothing about which cut is held.
///
/// Which cut is the slot's answer, `mix` or `exit`, written where the procedure
/// is instantiated rather than in the file — which is
/// [P-0086](../../../docs/principles/0086-a-procedure-knows-only-what-it-declares.md)
/// at the width of one declaration, and the same division `uses` and `edge`
/// already draw. `retains mix` in the file would be two procedures where there
/// is one, `feedback_mix.kir` and `feedback_exit.kir`, with the operator's
/// choice spelled as a library swap.
///
/// A struct rather than a `bool` so the span survives: a refusal about
/// `retains` on a kind that has no frames to retain has to point at the
/// declaration.
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
    /// The named inputs this procedure declares, of whichever types its kind
    /// allows.
    ///
    /// A geometry slot is L2's alone and breaks `L2 : Geometry -> Geometry` on the
    /// *arity* axis, two geometries in and one out, as `amplify` breaks it on the
    /// count axis. A Field slot is legal on the four kinds that can evaluate one
    /// and changes no signature at all: a field has no node, so naming one adds an
    /// input to the file and nothing to the chain.
    ///
    /// A list here and whatever the check pass allows after it. The parser collects
    /// every `uses` and decides nothing, so a declaration too many is refused with
    /// a sentence about the type it was written with rather than silently
    /// overwriting the first — see `check_header`. This is the system's named
    /// fan-in, and the notation is what the rest of it will be spelled with.
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
    /// L5: once per texel of the frame it is handed, and never more.
    ///
    /// This is [`Topology::Fullscreen`]'s shape with the camera taken out, which is
    /// what most of the rules about it are: there is no element, no viewpoint and
    /// no geometry, and the whole of the procedure is this one body over `src`.
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
