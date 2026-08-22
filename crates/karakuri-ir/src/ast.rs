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
    /// Geometry modulation: `Geometry -> Geometry`. **Stateless by rule**,
    /// compute. See `docs/ir-spec.md`, "L2 and L3".
    L2,
    /// The camera: `() -> Camera`, and `Geometry -> Camera` once one can read
    /// geometry. Compute, one invocation.
    ///
    /// **It produces state, not a matrix**, because the reason it exists at all
    /// is multiplicity: a weighted blend of two trajectories is meaningful on
    /// six numbers and meaningless on the matrices derived from them. See
    /// `docs/ir-spec.md`, "L2 and L3".
    L3,
    /// Rendering. Stateless, a render pipeline.
    L4,
    /// **A spatial function: `vec3 -> float`.** Code rather than data, and the
    /// only kind that lowers to no pass of its own.
    ///
    /// `docs/roadmap.md`'s L5 note says a `kind` is what a procedure *lowers
    /// to*, and that an L5 has no `kind` because it has no code to lower. This
    /// is the mirror: a field has *only* code to lower, so it has a file and no
    /// node. What it lowers to is a WGSL function spliced into whichever
    /// procedures evaluate it, which is why it needs no buffer, no pass and no
    /// position in the chain.
    ///
    /// One per Set, on the same terms as the camera: several would need naming,
    /// naming is fan-in, and `docs/roadmap.md` says fan-in arrives with
    /// multiple L1 sources and brings its notation with it.
    Field,
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

impl Kind {
    /// **Every kind, in the order a Set addresses them.**
    ///
    /// Here rather than spelled out at each site: four separate loops in
    /// `karakuri-engine` carried this list, and `Field` was added to none of
    /// them — so a field's `param` could be written but not bound, published,
    /// read back or saved. One list, and the next kind reaches every one of
    /// them by existing.
    pub const ALL: [Kind; 5] = [Kind::L1, Kind::L2, Kind::L3, Kind::L4, Kind::Field];
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

/// **How the fragments that land on one texel are combined**, declared on the
/// L4 header.
///
/// Unlike [`Topology`] this is a declaration and not an inference, and the
/// reason is worth keeping beside the enum: the two modes differ in how the
/// results of *identical* assignments are combined, not in what is assigned, so
/// there is nothing an L4 could write that would imply one over the other.
///
/// **The two read `color`'s alpha differently**, which is the part an author
/// has to know. Under [`Blend::Additive`] alpha is emission strength and is
/// allowed past 1.0 — it scales what a fragment adds. Under [`Blend::Weighted`]
/// it is *opacity*, and opacity above 1.0 is not a thing: the revealage a
/// weighted pass accumulates is `prod(1 - a)`, which stops meaning "what is
/// still visible behind this" the moment a term goes negative. The generated
/// shader clamps it, so the value an author can usefully write is `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Colour adds and nothing occludes. Needs no sorting, which is why it is
    /// where v0.2 started — and it is why everything this project rendered
    /// before `weighted` glowed.
    Additive,
    /// **Weighted blended OIT**: order-independent transparency, approximated.
    ///
    /// Each fragment contributes to a colour accumulation weighted by how near
    /// the eye it is, and to a running `prod(1 - a)` revealage; a resolve pass
    /// divides the first by its own weight sum and composites it against the
    /// second. Order independent, so it does not interact with compaction —
    /// which is the whole reason it is the successor to `additive` rather than
    /// depth sorting, at `capacity` elements per frame.
    ///
    /// It is an approximation, and where it is coarse is stated rather than
    /// hidden: the weight is a function of where a fragment sits between the
    /// camera's near and far planes, so material occupying a thin slice of a
    /// wide frustum gets near-equal weights and the result approaches a plain
    /// alpha-weighted average. That degradation is graceful — what still
    /// separates it from `additive` is that a weighted layer *occludes*.
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

    /// How this attribute is synthesised when a consumer needs it and nothing
    /// upstream emits it, or `None` where there is no rule.
    ///
    /// **The rule names what the engine has to carry, not what a pass has to
    /// run.** Both of the two are pure functions of the element and the clock
    /// once one extra value rides along with the element — a spawn instant, or
    /// last frame's position — so the synthesis is a substitution at the read
    /// site and there is no adapter *node* anywhere. That is cheaper than the
    /// alternative in the obvious way and in one less obvious one: a node would
    /// have to sit somewhere in the chain, and where it sat would be visible.
    pub fn derivation(self) -> Option<Derivation> {
        Some(match self {
            Attr::Age => Derivation::SinceBirth,
            Attr::Velocity => Derivation::FrameDifference(Attr::Position),
            _ => return None,
        })
    }

    /// Whether [`Attr::derivation`] has a rule for this attribute.
    pub fn is_derivable(self) -> bool {
        self.derivation().is_some()
    }
}

/// How an attribute nothing emits is synthesised.
///
/// **A closed set with two members**, because the two are what
/// `docs/ir-spec.md` settled and because each one costs a slot on every element
/// of any Set that needs it. Adding a third is adding a slot, which is the sort
/// of thing that should be hard rather than a matter of extending a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Derivation {
    /// `age`, as the clock minus the instant the element was spawned.
    ///
    /// Exact rather than accumulated, which is the difference between this and
    /// what an L1 writes by hand: an accumulated `age` drifts with the
    /// substep count and with the birth-fraction correction, and this does not.
    /// It costs the engine a `birth_t` slot, written once at spawn and carried.
    SinceBirth,
    /// `velocity`, as the change in a source attribute over one frame, divided
    /// by the step.
    ///
    /// **It costs a slot holding last frame's value, and that is what makes it
    /// survive compaction.** The obvious implementation — differencing the two
    /// buffers double buffering already has — does not work, because compaction
    /// moves elements between frames and index `i` is not the same element in
    /// both. A value carried *on* the element moves with it, exactly as
    /// `birth_frac` and `seed` do, and needs no map from an identity back to a
    /// slot.
    ///
    /// `docs/ir-spec.md` said this needed a third buffer. It does not; it needs
    /// a slot, which is the same answer the document already reaches for every
    /// other per-element value the engine owns.
    FrameDifference(Attr),
}

impl Derivation {
    /// The attribute this rule reads, where it has one.
    pub fn source(self) -> Option<Attr> {
        match self {
            Derivation::SinceBirth => None,
            Derivation::FrameDifference(from) => Some(from),
        }
    }

    /// **Whether the engine stores the derived attribute itself, or a reader
    /// synthesises it from something else the engine stored.**
    ///
    /// The two rules answer differently, and the reason is a decision this
    /// language already made rather than an implementation preference.
    ///
    /// `age` is `t` minus a stored spawn instant, and `t` is readable
    /// everywhere — so the cheaper half is stored and the subtraction happens
    /// where the attribute is read. Nothing has to be recomputed per frame and
    /// the answer is exact rather than accumulated.
    ///
    /// `velocity` is a difference over a step divided by that step, and **an L4
    /// deliberately has no `dt`**: a renderer is not given the simulation's step
    /// because it does not integrate. Synthesising it at the read site would
    /// mean smuggling `dt` into every renderer's uniform to serve one rule. So
    /// the division happens where the step already is — in the L1's own pass,
    /// against the same birth-fraction-corrected `dt` the simulation used — and
    /// what a reader sees is an ordinary stored attribute.
    pub fn is_stored(self) -> bool {
        match self {
            Derivation::SinceBirth => false,
            Derivation::FrameDifference(_) => true,
        }
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

    /// camera: where the camera is, in world space. **Required.**
    ///
    /// The same value a marching fragment reads as [`Ambient::Eye`] — one
    /// concept, written here and read there, the way [`Attr::Position`] is
    /// written by an L1 and read by an L4.
    Eye,
    /// camera: what it looks at, in world space. **Required.**
    ///
    /// A point rather than a direction, because pointing at a thing is what a
    /// camera in this system is asked to do — follow an element, hold the
    /// centre — and a direction would make every one of those a subtraction and
    /// a normalize the author had to remember.
    Target,
    /// camera: which way is up, in world space. Defaults to `+y`.
    Up,
    /// camera: vertical field of view, radians. Defaults to a third of pi.
    ///
    /// Vertical, and there is no horizontal one: the aspect ratio belongs to
    /// the canvas rather than to the camera, so what a frame shows sideways is
    /// decided where the frame is.
    FovY,
    /// camera: the near plane. Defaults to 0.1.
    Near,
    /// mask: **how much of this deformation applies to this element**, in
    /// `[0, 1]`. **Required** in a `mask` block.
    ///
    /// A scalar rather than a predicate, which is what makes it the same word
    /// the mixer already uses: `crate::deck`'s mask is a fader varying across
    /// the frame, and this is a modulator's varying across the material. A
    /// predicate is expressible — `step(0.7, age)` — and a soft boundary, which
    /// a spatial mask needs, is not expressible the other way round.
    Strength,
    /// field: **the signed distance at [`Ambient::Point`].** **Required** in a
    /// `field` block, and the whole of what a field produces.
    ///
    /// Signed rather than unsigned, and a distance rather than a density,
    /// because that is what every builtin in the SDF table returns and what the
    /// CSG operators compose. A field that returned something else would have
    /// no operators.
    Distance,
    /// camera: the far plane. Defaults to 100.
    ///
    /// **Not decorative.** `blend weighted` normalises a fragment's depth
    /// against [`Output::Near`] and this, so a camera that moves `far` moves
    /// every weighted fragment's weight with it.
    Far,
}

impl Output {
    pub const ALL: [Output; 12] = [
        Output::Clip,
        Output::ClipB,
        Output::PointSize,
        Output::Color,
        Output::Eye,
        Output::Target,
        Output::Up,
        Output::FovY,
        Output::Near,
        Output::Far,
        Output::Strength,
        Output::Distance,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Output::Clip => "clip",
            Output::ClipB => "clip_b",
            Output::PointSize => "point_size",
            Output::Color => "color",
            Output::Eye => "eye",
            Output::Target => "target",
            Output::Up => "up",
            Output::FovY => "fov_y",
            Output::Near => "near",
            Output::Far => "far",
            Output::Strength => "strength",
            Output::Distance => "distance",
        }
    }

    pub fn from_name(s: &str) -> Option<Output> {
        Output::ALL.into_iter().find(|o| o.name() == s)
    }

    pub fn ty(self) -> Ty {
        match self {
            Output::Clip | Output::ClipB | Output::Color => Ty::Vec4,
            Output::PointSize
            | Output::FovY
            | Output::Near
            | Output::Far
            | Output::Strength
            | Output::Distance => Ty::Float,
            Output::Eye | Output::Target | Output::Up => Ty::Vec3,
        }
    }

    pub fn block(self) -> BlockKind {
        match self {
            Output::Clip | Output::ClipB | Output::PointSize => BlockKind::Vertex,
            Output::Color => BlockKind::Fragment,
            Output::Eye
            | Output::Target
            | Output::Up
            | Output::FovY
            | Output::Near
            | Output::Far => BlockKind::Camera,
            Output::Strength => BlockKind::Mask,
            Output::Distance => BlockKind::Field,
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
    /// **Which copy of its parent this element is**, from the amplifying L2
    /// that made it — `0` where nothing upstream amplified.
    ///
    /// Identity downstream of an amplifier is the *pair* of the parent's `seed`
    /// and this index, and keeping them apart is what the pair buys:
    /// `hash1(seed)` still gives every mirror image of one element the same
    /// colour, which is what makes eight copies read as one object. Telling them
    /// apart is the deliberate `hash1(seed + copy * 8191u)`. The language has no
    /// bitwise operators, so the combination is arithmetic — a large odd
    /// multiplier, so that two copies of different parents do not collide.
    ///
    /// Stacked amplifiers compose it rather than overwrite it — a node of factor
    /// `n` turns a parent's `copy` into `copy * n + c` — so the index stays
    /// unique across the whole chain instead of only across the last stage.
    Copy,
    /// **Where the field is being evaluated**, in world space. `field` block
    /// only.
    ///
    /// The argument a caller hands over — `shape(p)` — seen from inside. It
    /// is an ambient rather
    /// than a declared parameter because a `.kir` procedure has no parameter
    /// list — every block reads its inputs by name, and this is that pattern
    /// with one input.
    Point,
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
    /// **The view-projection matrix**, for an L4 that projects an element.
    ///
    /// *Which* camera it is is the Set's answer and not the file's: it is the
    /// Set's camera, which is the first camera node — the one L3 procedure a
    /// Set used to be allowed, or the built-in orbit where it has none. A
    /// renderer in a Set of several that wants one of the others declares a
    /// slot and reads `view.clip` through it; see [`SlotTy::Camera`], which
    /// this and the two below are the unnamed form of.
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
    /// **Which geometry this chain instance is running over**, as the assigned
    /// value that identifies it — its salt, which `docs/ir-spec.md` settles is
    /// the value rather than a dense index beside it.
    ///
    /// **A per-source uniform and not an element attribute**, which is the one
    /// thing about it a reader is most likely to have backwards: the spec's
    /// table called it "implicit, carried, never declared" and the engine
    /// reached the other answer. A Set instantiates its chain **per source**,
    /// so an instance knows *statically* which geometry it runs over — and a
    /// value that is the same for every element an instance will ever touch is
    /// a uniform by definition. Carrying it on the element instead would be a
    /// `u32` on every element of every merged Set spent restating a constant.
    ///
    /// So nothing new is written to read it: `Source::salt` is per geometry,
    /// `Set::prepare` already writes it as `seed_salt` into every node's
    /// uniform block, and every generated module already declares the field.
    /// The read is the whole of what was missing.
    ///
    /// It is what a downstream mask compares against, and what it compares
    /// *to* is a [`SlotTy::Source`] slot — a `.kir` may not name a node, so
    /// the comparand arrives through a slot the Set binds.
    Source,
}

impl Ambient {
    pub const ALL: [Ambient; 12] = [
        Ambient::Seed,
        Ambient::Source,
        Ambient::Copy,
        Ambient::Point,
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
            Ambient::Source => "source",
            Ambient::Copy => "copy",
            Ambient::Point => "point",
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
            Ambient::Seed | Ambient::Source | Ambient::Copy | Ambient::Capacity => Ty::Uint,
            Ambient::T | Ambient::Beats | Ambient::Dt => Ty::Float,
            Ambient::Camera => Ty::Mat4,
            Ambient::Eye | Ambient::Ray | Ambient::Point => Ty::Vec3,
            Ambient::PointCoord => Ty::Vec2,
        }
    }

    /// Blocks this value is readable in. `Seed` is readable everywhere.
    pub fn available_in(self, kind: Kind, block: BlockKind) -> bool {
        match self {
            // **Not in a `camera` block**, which has no element to have one.
            // **Two kinds have no element, and `seed` is refused in both.** An
            // L3 runs once a frame over nothing, and the salt the engine mixes
            // into `seed` is a property of a Set's geometry, which a camera is
            // not. A field is a function of space: it is handed a `point`, it
            // is spliced into whichever procedures call `field(p)`, and one of
            // those may be a fragment stage — so there is no element even in
            // principle, let alone one this evaluation belongs to.
            Ambient::Seed => kind != Kind::L3 && kind != Kind::Field,
            // **The same two kinds, and it is the same sentence.** An L3 runs
            // once a frame over nothing and the salt is a property of a Set's
            // geometry, which a camera is not; a field is a function of space,
            // spliced into whichever procedures call it — one of which may be
            // a fragment stage — so there is no element and no chain instance
            // even in principle, let alone one this evaluation belongs to.
            //
            // **Not on `seed`'s other terms, though.** `seed` is per element
            // and this is per *instance*, so a fullscreen L4 — which has no
            // element and reads no `seed` — reads this perfectly well: its
            // uniform holds the salt like every other module's.
            Ambient::Source => kind != Kind::L3 && kind != Kind::Field,
            // **Downstream of an amplifier, and nowhere else it could mean
            // anything.** An L1 writes the buffer an amplifier later reads, so
            // `copy` there is zero by construction; an L3 has no element. In an
            // L2 or an L4 it is either the index of the copy this invocation is
            // producing or the one an upstream node produced, and where nothing
            // upstream amplified it is zero — the same answer `seed` gives in a
            // procedure that spawns nothing, and for the same reason.
            Ambient::Copy => kind == Kind::L2 || kind == Kind::L4,
            // **The one input a field has, and nowhere else has one.** Every
            // other block is handed an element or a fragment; a field is handed
            // a position and nothing else, which is what makes it a function of
            // space rather than of the material in it.
            Ambient::Point => block == BlockKind::Field,
            Ambient::T | Ambient::Beats => true,
            Ambient::Capacity => kind == Kind::L1,
            // **An L3 gets `dt` and an L4 does not**, which is the asymmetry
            // between a node that may hold state and one that may not: a
            // camera's craft is mostly smoothing, and smoothing is written
            // against a step. See `docs/ir-spec.md`, "L2 and L3".
            Ambient::Dt => kind == Kind::L1 || kind == Kind::L3,
            Ambient::Camera => kind == Kind::L4,
            Ambient::PointCoord => block == BlockKind::Fragment,
            // Fragment-only, and not because a vertex stage could not be given
            // them: a fullscreen procedure has no vertex block at all, and in a
            // per-element one a ray through the fragment is not a thing a
            // vertex has. Offering them where they mean nothing would be one
            // more way to write a procedure that compiles and is wrong.
            Ambient::Eye | Ambient::Ray => kind == Kind::L4 && block == BlockKind::Fragment,
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

impl Param {
    /// **The declared default as a number**, or `None` where the declaration is
    /// an expression this does not fold.
    ///
    /// **A negation is folded, because the parser does not fold it.** `= -0.35`
    /// is `Unary { Neg, Lit }` and not a literal, so matching [`Expr::Lit`]
    /// alone silently dropped every negative default: the engine's param never
    /// entered its uniform, its declared value was discarded, binding refused
    /// it, and the shader got whatever the miss produced — a panic on the render
    /// thread before that reader returned an `Option`, and a quiet `0.0` after.
    /// A `.kir` declaring `param drift : float [-1.0, 1.0] = -0.35` is legal and
    /// none of that is a reader's to decide.
    ///
    /// **Not general constant folding**, deliberately. A default is checked in
    /// an empty scope, so it is *some* constant, but the useful set is one
    /// literal with an optional sign in front of it. Widening it is a language
    /// question — what a default may say — rather than a convenience for one
    /// caller, and it belongs here where every caller gets the same answer.
    ///
    /// **It lives here rather than in a reader, and that is the whole point.**
    /// It was private to `karakuri-engine`'s `Set`, with a note saying a second
    /// evaluator elsewhere would agree with the shader only by coincidence.
    /// There is now a second reader — `karakuri-cli`'s metadata writer records
    /// this number in a `param_decl` — and a metadata file whose `default`
    /// disagreed with the uniform the run actually loaded would be a card
    /// describing a procedure nobody ran. One function, so they cannot differ.
    ///
    /// `None` is *"this default is not a number I can state"*, and never
    /// *"there is no default"*: the grammar makes `= <expr>` mandatory. What a
    /// caller does with that is the caller's — the engine leaves the param out
    /// of its uniform, the metadata writer writes the declaration with no
    /// `default` key.
    pub fn default_scalar(&self) -> Option<f32> {
        match &self.default {
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

/// `amplify <factor>`
///
/// **A compile-time constant, on the same terms as a loop bound**, because the
/// output buffer is sized from it and the cost is multiplied out by it — neither
/// of which a runtime value could do. It is the one thing in the language that
/// changes an element count, which is why it is a header declaration rather than
/// anything a block can say: what a `deform` writes is decided before it runs.
#[derive(Debug, Clone, Copy)]
pub struct AmplifyDecl {
    pub factor: u32,
    pub span: Span,
}

/// `uses <name> : Geometry`, `: Field`, `: Camera`, `: Source`
///
/// **One input this node takes, named by the procedure and bound by the Set.**
///
/// The name is the *procedure's own*, exactly as `consumes position` names an
/// attribute without naming which L1 supplies it. That is what keeps a `.kir` a
/// library part: a file that named a node would be coupled to one Set and could
/// not be used in another. What fills the slot is written where the use is
/// recorded — an `edge` in the Set file, `--edge <node>.<slot>=<node>` on the
/// command line — and an unbound slot is refused rather than filled in from
/// whatever happened to be lying around.
///
/// **The type decides every rule about it**, which is why it is written and
/// carried rather than checked and dropped — see [`SlotTy`]. Which kinds may
/// declare one, how many are legal, what an `edge` may bind it to and how it is
/// read all differ between the four, and each of those refusals is a sentence
/// about a type rather than about `uses`.
#[derive(Debug, Clone)]
pub struct UsesDecl {
    pub name: String,
    /// The name alone, so a refusal about what it collides with points at it
    /// rather than at the whole declaration.
    pub name_span: Span,
    /// What the slot takes, as the header spelled it.
    pub ty: SlotTy,
    pub span: Span,
}

/// **What a `uses` slot takes** — the type in `uses far : Geometry`.
///
/// **The value is what the rules are about.** "An L3 produces a viewpoint, not
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
    /// **A `kind Field` procedure, evaluated at a point.**
    ///
    /// Read as a *call* — `shape(p)` — because a field has no members and does
    /// have an argument. That reuses `Expr::Call` the way a geometry slot's
    /// read reuses `expr . ident`: one existing shape given a second meaning,
    /// resolved against what the header declared rather than against a word the
    /// language reserved. Reserving one is what capped a procedure at one
    /// field, and there is no reserved word left to cap it.
    Field,
    /// **A viewpoint this renderer draws from** — a `kind L3` procedure, or the
    /// built-in orbit, which is a node like any other so that an edge can name
    /// it.
    ///
    /// Read as a *member* — `view.clip`, `view.eye`, `view.ray` — because a
    /// camera is neither elements nor a function: it is one value with parts,
    /// and the parts are the three derivations a renderer is handed. That
    /// reuses `expr . ident` the way a geometry slot's read does, with one
    /// difference that is the whole of what this variant cost the checker: the
    /// members are resolved against the slot's *type*, where `far.position`
    /// resolves against the attribute table.
    ///
    /// **The ambients it stands in for stay.** `camera`, `eye` and `ray` are
    /// the Set's camera for a renderer that declares no slot — which is every
    /// renderer written before this notation existed — and a renderer that
    /// declares one is saying *which* camera, which is the question a Set with
    /// several has no other way to be asked.
    Camera,
    /// **The identity of one geometry, for comparing [`Ambient::Source`]
    /// against** — the L1 an edge names, reduced to the assigned value that
    /// identifies it.
    ///
    /// **It is not `Geometry`, and the difference is what it binds.** A
    /// `Geometry` slot binds an element *buffer* — a bind-group entry on every
    /// node that has one — and is capped at one because a second bound buffer
    /// is not built. This binds a `u32` in a uniform the module already has. A
    /// mask wants the identity and reads no elements, so declaring it
    /// `Geometry` would both allocate a buffer nothing reads and collide with
    /// that cap on a node which already declares a `far`.
    ///
    /// Read as a **value**, alone, which no other slot type is: `only` is the
    /// bound source's identity. `docs/ir-spec.md` says a geometry is not a
    /// value because "the language has no type for a whole source and no way to
    /// pass one" — that sentence is about `Geometry` and stays true. This is a
    /// `uint`, which the language does have.
    ///
    /// **Several are legal**, unlike `Geometry` and `Camera`: a mask that says
    /// `source == a || source == b` is the ordinary case, and each slot costs
    /// one `u32` in a uniform block rather than a buffer or a bind group.
    Source,
}

impl SlotTy {
    /// The spelling a header uses, on [`Attr::from_name`]'s terms: one place
    /// that knows which words are types, so a third type is a line here and
    /// nothing in the parser.
    pub fn from_name(s: &str) -> Option<SlotTy> {
        Some(match s {
            "Geometry" => SlotTy::Geometry,
            "Field" => SlotTy::Field,
            "Camera" => SlotTy::Camera,
            "Source" => SlotTy::Source,
            _ => return None,
        })
    }

    /// The spelling, back — for a refusal that has to name the type the header
    /// was written with rather than the one it is talking about.
    pub fn name(self) -> &'static str {
        match self {
            SlotTy::Geometry => "Geometry",
            SlotTy::Field => "Field",
            SlotTy::Camera => "Camera",
            SlotTy::Source => "Source",
        }
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
    /// L2 only.
    pub amplify: Option<AmplifyDecl>,
    /// **The named inputs this procedure declares**, of whichever types its
    /// kind allows.
    ///
    /// A geometry slot is L2's alone and breaks `L2 : Geometry -> Geometry` on
    /// the *arity* axis, two geometries in and one out, as `amplify` breaks it
    /// on the count axis. A Field slot is legal on the four kinds that can
    /// evaluate one and changes no signature at all: a field has no node, so
    /// naming one adds an input to the file and nothing to the chain.
    ///
    /// **A list here and whatever the check pass allows after it.** The parser
    /// collects every `uses` and decides nothing, so a declaration too many is
    /// refused with a sentence about the type it was written with rather than
    /// silently overwriting the first — see `check_header`. This is the
    /// system's named fan-in, and the notation is what the rest of it will be
    /// spelled with.
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
    /// L2: rewrite a live element's attributes. **May not `kill()`** — see
    /// [`Kind::L2`].
    Deform,
    /// L2: **where** this deformation applies, and how much. Optional; an L2
    /// with no `mask` block applies everywhere at full strength.
    Mask,
    /// L3: produce this frame's camera state.
    Camera,
    /// Field: the signed distance at [`Ambient::Point`]. The whole of a field
    /// procedure, and the only block whose lowering is a WGSL function rather
    /// than an entry point.
    Field,
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
            BlockKind::Deform => "deform",
            BlockKind::Mask => "mask",
            BlockKind::Camera => "camera",
            BlockKind::Vertex => "vertex",
            BlockKind::Fragment => "fragment",
            BlockKind::Field => "field",
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
