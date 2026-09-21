#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Geometry generation. Stateful, compute.
    L1,
    /// Geometry modulation: `Geometry -> Geometry`. Stateless by rule, compute. See
    /// `docs/ir-spec.md`, "L2 and L3".
    L2,
    /// The camera: `() -> Camera`, and `Geometry -> Camera` once one can read
    /// geometry. Compute, one invocation.
    ///
    /// It produces state, not a matrix, because the reason it exists at all is
    /// multiplicity: a weighted blend of two trajectories is meaningful on six
    /// numbers and meaningless on the matrices derived from them. See
    /// `docs/ir-spec.md`, "L2 and L3".
    L3,
    /// Rendering. Stateless, a render pipeline.
    L4,
    /// A spatial function: `vec3 -> float`. Code rather than data, and the only
    /// kind that lowers to no pass of its own.
    ///
    /// A `kind` is what a procedure *lowers to*, and an L5 has no `kind` because it
    /// has no code to lower — `docs/ir-spec.md`, "kind". This is the mirror: a
    /// field has *only* code to lower, so it has a file and no node. What it lowers
    /// to is a WGSL function spliced into whichever procedures evaluate it, which
    /// is why it needs no buffer, no pass and no position in the chain.
    ///
    /// One per Set no longer, and on the same terms as the camera: a Set holds as
    /// many fields as its files declare, each a node with a name an edge can point
    /// at. The cap was the missing notation rather than the language — a caller
    /// reaches one through a slot its own procedure declares, `uses shape : Field`
    /// called `shape(p)` — which is the fan-in multiple L1 sources arrived with.
    /// See `docs/adr/0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md`.
    Field,
    /// A frame effect: `[Texture] -> Texture`. One required block,
    /// [`BlockKind::Frame`], which is a fullscreen fragment body over the incoming
    /// texture.
    ///
    /// One kind with two roles, and what differs between them is only whether a
    /// surface is attached: a master chain slot is this signature with one input,
    /// and a nested merge is the same with several, bound by `edge`s to as many
    /// `uses … : Texture` slots as the header declares. See
    /// `docs/adr/0098-l5-is-one-node-kind-with-two-roles.md`.
    ///
    /// The absence this ends was a condition rather than a principle. A `kind` says
    /// what a procedure *lowers to*, and while the compositing was fixed there was
    /// nothing for a `kind L5` file to contain. Somebody wrote the compositing down
    /// —
    /// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`
    /// — so there is code to lower and the layer algebra gains a line rather than
    /// an exception. `crate::node::Merge` keeps its place beside this kind exactly
    /// where the built-in orbit camera keeps its place beside [`Kind::L3`].
    L5,
}

/// What a procedure's geometry *is*, on an L1 header, and what an L4 procedure
/// *draws*, inferred rather than declared — see
/// [`crate::typed::Checked::topology`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topology {
    /// One sprite per element.
    Points,
    /// One segment per element, from [`Output::Clip`] to [`Output::ClipB`].
    ///
    /// The connectivity is deliberately not a graph. An element nominating *another
    /// element* as its far end would be an index into the element buffer, and
    /// compaction moves elements between steps — so the one shape that would
    /// express a strip is also the one that spawning material invalidates. A
    /// segment whose two ends both belong to one element has no such dependency: a
    /// polyline is *n* segments, a trail is one segment per particle, and both
    /// survive compaction because neither refers to anything outside itself.
    ///
    /// It costs the duplication of shared endpoints — a strip of *n* samples stores
    /// 2*n* points rather than *n*+1. That is the price of the primitive being
    /// smaller than the gesture.
    Lines,
    /// The whole frame, once, with no geometry at all.
    ///
    /// Inferred for an L4 that has no `vertex` block, because a procedure with no
    /// per-element position has nothing for one to do. Refused on an L1: geometry
    /// cannot be fullscreen, and the value exists on this enum only because an L4's
    /// answer and an L1's declaration share a field.
    ///
    /// Such a procedure consumes nothing — there is nowhere to read an element from
    /// — which is what lets the engine skip the paired L1's simulation entirely
    /// rather than run it for a reader that does not exist.
    Fullscreen,
}

impl Kind {
    /// Every kind, in the order a Set addresses them.
    ///
    /// Here rather than spelled out at each site: four separate loops in
    /// `karakuri-engine` carried this list, and `Field` was added to none of them —
    /// so a field's `param` could be written but not bound, published, read back or
    /// saved. One list, and the next kind reaches every one of them by existing.
    pub const ALL: [Kind; 6] = [
        Kind::L1,
        Kind::L2,
        Kind::L3,
        Kind::L4,
        Kind::Field,
        Kind::L5,
    ];

    /// Name of the layer kind as used in source declarations and error messages.
    pub const fn name(&self) -> &'static str {
        match self {
            Kind::L1 => "L1",
            Kind::L2 => "L2",
            Kind::L3 => "L3",
            Kind::L4 => "L4",
            Kind::Field => "Field",
            Kind::L5 => "L5",
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// An error returned when parsing a [`Kind`] from a string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseKindError(String);

impl std::fmt::Display for ParseKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown layer kind: `{}`; expected one of L1, L2, L3, L4, Field, L5",
            self.0
        )
    }
}

impl std::error::Error for ParseKindError {}

impl std::str::FromStr for Kind {
    type Err = ParseKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "L1" => Ok(Kind::L1),
            "L2" => Ok(Kind::L2),
            "L3" => Ok(Kind::L3),
            "L4" => Ok(Kind::L4),
            "FIELD" => Ok(Kind::Field),
            "L5" => Ok(Kind::L5),
            _ => Err(ParseKindError(s.to_string())),
        }
    }
}

/// The incoming texture an L5 is handed, reserved in a `frame` block the way
/// [`Output::Color`] is reserved in a `fragment` one.
///
/// Implicit rather than declared: what supplies it is the chain slot's position
/// or the Set's `edge`s, and a procedure that named its own supplier would be
/// coupled to one arrangement.
pub const TEXTURE_SRC: &str = "src";

/// The retained cut of the previous frame, readable only where the header
/// declares `retains`.
///
/// *Which* cut is not the procedure's to know — the slot answers that with
/// `mix` or `exit` — which is the same division `uses` and `edge` already draw.
pub const TEXTURE_HELD: &str = "held";

impl Topology {
    pub fn name(self) -> &'static str {
        match self {
            Topology::Points => "points",
            Topology::Lines => "lines",
            Topology::Fullscreen => "fullscreen",
        }
    }
}

/// How the fragments that land on one texel are combined, declared on the L4
/// header.
///
/// Unlike [`Topology`] this is a declaration and not an inference, and the
/// reason is worth keeping beside the enum: the two modes differ in how the
/// results of *identical* assignments are combined, not in what is assigned, so
/// there is nothing an L4 could write that would imply one over the other.
///
/// The two read `color`'s alpha differently, which is the part an author has to
/// know. Under [`Blend::Additive`] alpha is emission strength and is allowed
/// past 1.0 — it scales what a fragment adds. Under [`Blend::Weighted`] it is
/// *opacity*, and opacity above 1.0 is not a thing: the revealage a weighted
/// pass accumulates is `prod(1 - a)`, which stops meaning "what is still
/// visible behind this" the moment a term goes negative. The generated shader
/// clamps it, so the value an author can usefully write is `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Colour adds and nothing occludes. Needs no sorting, which is why it is where
    /// v0.2 started — and it is why everything this project rendered before
    /// `weighted` glowed.
    Additive,
    /// Weighted blended OIT: order-independent transparency, approximated.
    ///
    /// Each fragment contributes to a colour accumulation weighted by how near the
    /// eye it is, and to a running `prod(1 - a)` revealage; a resolve pass divides
    /// the first by its own weight sum and composites it against the second. Order
    /// independent, so it does not interact with compaction — which is the whole
    /// reason it is the successor to `additive` rather than depth sorting, at
    /// `capacity` elements per frame.
    ///
    /// It is an approximation, and where it is coarse is stated rather than hidden:
    /// the weight is a function of where a fragment sits between the camera's near
    /// and far planes, so material occupying a thin slice of a wide frustum gets
    /// near-equal weights and the result approaches a plain alpha-weighted average.
    /// That degradation is graceful — what still separates it from `additive` is
    /// that a weighted layer *occludes*.
    Weighted,
}

impl Blend {
    pub fn name(self) -> &'static str {
        match self {
            Blend::Additive => "additive",
            Blend::Weighted => "weighted",
        }
    }
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

    /// How many `f32` values a `param` of this type is driven as, or `None` for a
    /// type no `param` may declare.
    ///
    /// Deliberately narrower than [`Ty::components`], and the narrowness is the
    /// point: that one answers for every type in the language, where this answers
    /// the checker's own list — *"params may only be `float`, `vec2`, or `vec3`"*
    /// (`check.rs`). A `vec4` param does not exist, so a caller spelling out its
    /// components would be spelling out a declaration nothing can make; `None` says
    /// that rather than inventing a fourth key.
    pub fn param_components(self) -> Option<usize> {
        Some(match self {
            Ty::Float => 1,
            Ty::Vec2 => 2,
            Ty::Vec3 => 3,
            _ => return None,
        })
    }
}

/// What a `uses` slot takes — the type in `uses far : Geometry`.
///
/// The value is what the rules are about. "An L3 produces a viewpoint, not
/// geometry" is a sentence about a *geometry* slot rather than about `uses`,
/// and a checker matching on this says so — which is why the second variant
/// cost each such refusal one arm rather than a rewrite around a distinction
/// nothing had drawn.
///
/// The four differ in every rule that mentions them, which is the argument for
/// the type being written down at all: a geometry slot is L2's alone and there
/// is at most one, because a second bound element buffer is not built; a Field
/// slot is legal on the four kinds that can evaluate one and there may be
/// several, because a marcher wanting a shape and a cutter is the ordinary
/// case; a Camera slot is L4's alone and there is at most one, because a
/// renderer draws one picture and a picture is seen from one place; a Source
/// slot is legal wherever a chain instance runs and there may be several,
/// because `source == a || source == b` is an ordinary thing to want.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotTy {
    /// The elements of an L1, read beside the ones this node runs over.
    Geometry,
    /// A `kind Field` procedure, evaluated at a point.
    ///
    /// Read as a *call* — `shape(p)` — because a field has no members and does have
    /// an argument. That reuses `Expr::Call` the way a geometry slot's read reuses
    /// `expr . ident`: one existing shape given a second meaning, resolved against
    /// what the header declared rather than against a word the language reserved.
    /// Reserving one is what capped a procedure at one field, and there is no
    /// reserved word left to cap it.
    Field,
    /// A viewpoint this renderer draws from — a `kind L3` procedure, or the
    /// built-in orbit, which is a node like any other so that an edge can name it.
    ///
    /// Read as a *member* — `view.clip`, `view.eye`, `view.ray` — because a camera
    /// is neither elements nor a function: it is one value with parts, and the
    /// parts are the three derivations a renderer is handed. That reuses `expr .
    /// ident` the way a geometry slot's read does, with one difference that is the
    /// whole of what this variant cost the checker: the members are resolved
    /// against the slot's *type*, where `far.position` resolves against the
    /// attribute table.
    ///
    /// The ambients it stands in for stay. `camera`, `eye` and `ray` are the Set's
    /// camera for a renderer that declares no slot — which is every renderer
    /// written before this notation existed — and a renderer that declares one is
    /// saying *which* camera, which is the question a Set with several has no other
    /// way to be asked.
    Camera,
    /// The identity of one geometry, for comparing [`Ambient::Source`] against —
    /// the L1 an edge names, reduced to the assigned value that identifies it.
    ///
    /// It is not `Geometry`, and the difference is what it binds. A `Geometry` slot
    /// binds an element *buffer* — a bind-group entry on every node that has one —
    /// and is capped at one because a second bound buffer is not built. This binds
    /// a `u32` in a uniform the module already has. A mask wants the identity and
    /// reads no elements, so declaring it `Geometry` would both allocate a buffer
    /// nothing reads and collide with that cap on a node which already declares a
    /// `far`.
    ///
    /// Read as a value, alone, which no other slot type is: `only` is the bound
    /// source's identity. `docs/ir-spec.md` says a geometry is not a value because
    /// "the language has no type for a whole source and no way to pass one" — that
    /// sentence is about `Geometry` and stays true. This is a `uint`, which the
    /// language does have.
    ///
    /// Several are legal, unlike `Geometry` and `Camera`: a mask that says `source
    /// == a || source == b` is the ordinary case, and each slot costs one `u32` in
    /// a uniform block rather than a buffer or a bind group.
    Source,
    /// A picture this node folds in — one input of a nested [`Kind::L5`], bound by
    /// an `edge` like every other slot.
    ///
    /// Legal on an L5 and nowhere else, any number of them: this is the nested
    /// role's fan-in, and it is the fan-in `docs/architecture.md` says is already
    /// solved — `uses` plus `edge`, with no new mechanism. Every other kind is
    /// handed elements or a position, and a texture is neither.
    ///
    /// Read through the two texture builtins, `texel(<slot>)` and `tap(<slot>,
    /// uv)`, which is what separates it from every slot type beside it: a geometry
    /// is read as a member, a field as a call, a source as a bare value, and a
    /// texture only ever as a fetch. There is no [`Ty`] for one, deliberately — a
    /// texture is not a value this language can hold, so `let x = tex;` has nothing
    /// to bind and is refused where it is written.
    ///
    /// A chain slot's L5 declares none. The master chain is an ordered list and its
    /// only fan-in is that order; there is no Set for an `edge` to be written in,
    /// so a procedure with a Texture slot is refused *from a chain slot* rather
    /// than from the language — a refusal that belongs where the chain is built,
    /// beside the one that refuses an unbound slot.
    Texture,
}

impl SlotTy {
    /// The spelling a header uses, on [`Attr::from_name`]'s terms: one place that
    /// knows which words are types, so a third type is a line here and nothing in
    /// the parser.
    pub fn from_name(s: &str) -> Option<SlotTy> {
        Some(match s {
            "Geometry" => SlotTy::Geometry,
            "Field" => SlotTy::Field,
            "Camera" => SlotTy::Camera,
            "Source" => SlotTy::Source,
            "Texture" => SlotTy::Texture,
            _ => return None,
        })
    }

    /// The spelling, back — for a refusal that has to name the type the header was
    /// written with rather than the one it is talking about.
    pub fn name(self) -> &'static str {
        match self {
            SlotTy::Geometry => "Geometry",
            SlotTy::Field => "Field",
            SlotTy::Camera => "Camera",
            SlotTy::Source => "Source",
            SlotTy::Texture => "Texture",
        }
    }
}
