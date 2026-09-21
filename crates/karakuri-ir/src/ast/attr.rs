use super::{BlockKind, Kind, Ty};

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
    /// The rule names what the engine has to carry, not what a pass has to run.
    /// Both of the two are pure functions of the element and the clock once one
    /// extra value rides along with the element — a spawn instant, or last frame's
    /// position — so the synthesis is a substitution at the read site and there is
    /// no adapter *node* anywhere. That is cheaper than the alternative in the
    /// obvious way and in one less obvious one: a node would have to sit somewhere
    /// in the chain, and where it sat would be visible.
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
/// A closed set with two members, because the two are what `docs/ir-spec.md`
/// settled and because each one costs a slot on every element of any Set that
/// needs it. Adding a third is adding a slot, which is the sort of thing that
/// should be hard rather than a matter of extending a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Derivation {
    /// `age`, as the clock minus the instant the element was spawned.
    ///
    /// Exact rather than accumulated, which is the difference between this and what
    /// an L1 writes by hand: an accumulated `age` drifts with the substep count and
    /// with the birth-fraction correction, and this does not. It costs the engine a
    /// `birth_t` slot, written once at spawn and carried.
    SinceBirth,
    /// `velocity`, as the change in a source attribute over one frame, divided by
    /// the step.
    ///
    /// It costs a slot holding last frame's value, and that is what makes it
    /// survive compaction. The obvious implementation — differencing the two
    /// buffers double buffering already has — does not work, because compaction
    /// moves elements between frames and index `i` is not the same element in both.
    /// A value carried *on* the element moves with it, exactly as `birth_frac` and
    /// `seed` do, and needs no map from an identity back to a slot.
    ///
    /// `docs/ir-spec.md` said this needed a third buffer. It does not; it needs a
    /// slot, which is the same answer the document already reaches for every other
    /// per-element value the engine owns.
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

    /// Whether the engine stores the derived attribute itself, or a reader
    /// synthesises it from something else the engine stored.
    ///
    /// The two rules answer differently, and the reason is a decision this language
    /// already made rather than an implementation preference.
    ///
    /// `age` is `t` minus a stored spawn instant, and `t` is readable everywhere —
    /// so the cheaper half is stored and the subtraction happens where the
    /// attribute is read. Nothing has to be recomputed per frame and the answer is
    /// exact rather than accumulated.
    ///
    /// `velocity` is a difference over a step divided by that step, and an L4
    /// deliberately has no `dt`: a renderer is not given the simulation's step
    /// because it does not integrate. Synthesising it at the read site would mean
    /// smuggling `dt` into every renderer's uniform to serve one rule. So the
    /// division happens where the step already is — in the L1's own pass, against
    /// the same birth-fraction-corrected `dt` the simulation used — and what a
    /// reader sees is an ordinary stored attribute.
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
    /// vertex: a segment's far end, in clip space.
    ///
    /// Optional, and the only optional output. Assigning it is how an L4 procedure
    /// says it draws [`Topology::Lines`] — there is nothing else a second endpoint
    /// could mean, so it is inferred rather than declared twice. Assigning it on
    /// *some* paths is an error: a procedure either draws segments or it does not.
    ClipB,
    /// vertex: point sprite size, as a fraction of the render target's height.
    /// Under [`Topology::Lines`], the width of the stroke, uniform along the
    /// segment, in the same unit.
    ///
    /// Not pixels, and the name says so. In shader work `point_rate` on a point
    /// sprite means pixels, and a shader would normally adjust it for the screen
    /// itself. This language keeps the render size away from a procedure, so the
    /// spelling has to say the number is not a pixel count.
    ///
    /// A fraction of the height rather than of the width, so that a change of
    /// aspect ratio does not resize a sprite. Width would.
    PointRate,
    /// fragment: linear RGB, straight alpha.
    Color,

    /// camera: where the camera is, in world space. Required.
    ///
    /// The same value a marching fragment reads as [`Ambient::Eye`] — one concept,
    /// written here and read there, the way [`Attr::Position`] is written by an L1
    /// and read by an L4.
    Eye,
    /// camera: what it looks at, in world space. Required.
    ///
    /// A point rather than a direction, because pointing at a thing is what a
    /// camera in this system is asked to do — follow an element, hold the centre —
    /// and a direction would make every one of those a subtraction and a normalize
    /// the author had to remember.
    Target,
    /// camera: which way is up, in world space. Defaults to `+y`.
    Up,
    /// camera: vertical field of view, radians. Defaults to a third of pi.
    ///
    /// Vertical, and there is no horizontal one: the aspect ratio belongs to the
    /// canvas rather than to the camera, so what a frame shows sideways is decided
    /// where the frame is.
    FovY,
    /// camera: the near plane. Defaults to 0.1.
    Near,
    /// mask: how much of this deformation applies to this element, in `[0, 1]`.
    /// Required in a `mask` block.
    ///
    /// A scalar rather than a predicate, which is what makes it the same word the
    /// mixer already uses: `crate::deck`'s mask is a fader varying across the
    /// frame, and this is a modulator's varying across the material. A predicate is
    /// expressible — `step(0.7, age)` — and a soft boundary, which a spatial mask
    /// needs, is not expressible the other way round.
    Strength,
    /// field: the signed distance at [`Ambient::Point`]. Required in a `field`
    /// block, and the whole of what a field produces.
    ///
    /// Signed rather than unsigned, and a distance rather than a density, because
    /// that is what every builtin in the SDF table returns and what the CSG
    /// operators compose. A field that returned something else would have no
    /// operators.
    Distance,
    /// camera: the far plane. Defaults to 100.
    ///
    /// Not decorative. `blend weighted` normalises a fragment's depth against
    /// [`Output::Near`] and this, so a camera that moves `far` moves every weighted
    /// fragment's weight with it.
    Far,
}

impl Output {
    pub const ALL: [Output; 12] = [
        Output::Clip,
        Output::ClipB,
        Output::PointRate,
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
            Output::PointRate => "point_rate",
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
            Output::PointRate
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
            Output::Clip | Output::ClipB | Output::PointRate => BlockKind::Vertex,
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
    /// Copy index of the current element produced by an upstream amplifying L2 node (`0` if unamplified).
    ///
    /// Composed hierarchically across stacked amplifiers (`copy * n + c`).
    Copy,
    /// Where the field is being evaluated, in world space. `field` block only.
    ///
    /// The argument a caller hands over — `shape(p)` — seen from inside. It is an
    /// ambient rather than a declared parameter because a `.kir` procedure has no
    /// parameter list — every block reads its inputs by name, and this is that
    /// pattern with one input.
    Point,
    Capacity,
    T,
    /// Musical position, in beats, on the session's tempo grid — the same instant
    /// [`Ambient::T`] names, counted in beats instead of seconds.
    ///
    /// Monotone while the tempo is positive and continuous across a tempo
    /// correction, which is what makes it safe to drive a phase from: a tempo
    /// change bends the rate and never moves a beat that has already happened.
    /// Fractional, so `fract(beats)` is the position within the beat and `beats *
    /// 0.25` counts bars in four.
    ///
    /// Not a substitute for `t` and not a replacement for it. `t` is the
    /// simulation's own clock and is what an accumulating procedure integrates
    /// against; `beats` is where the room is. A procedure that wants to move with
    /// the music reads this, and one that wants to move at a rate reads `t`.
    Beats,
    Dt,
    /// The view-projection matrix, for an L4 that projects an element.
    ///
    /// *Which* camera it is is the Set's answer and not the file's: it is the Set's
    /// camera, which is the first camera node — the one L3 procedure a Set used to
    /// be allowed, or the built-in orbit where it has none. A renderer in a Set of
    /// several that wants one of the others declares a slot and reads `view.clip`
    /// through it; see [`SlotTy::Camera`], which this and the two below are the
    /// unnamed form of.
    Camera,
    PointCoord,
    /// Where the camera is, in world space. L4 fragment only.
    ///
    /// One half of what a fullscreen procedure needs to march: a ray is an origin
    /// and a direction, and the origin is the same for every fragment.
    Eye,
    /// The unit direction from [`Ambient::Eye`] through this fragment, in world
    /// space. L4 fragment only.
    ///
    /// Given rather than derived, because the camera is a built-in here: a
    /// procedure that built its own would be restating the engine's projection
    /// convention, and getting it slightly wrong is a picture that looks nearly
    /// right. It also keeps a `mat4` inverse out of a language that has no operator
    /// for one.
    Ray,
    /// Which geometry this chain instance is running over, as the assigned value
    /// that identifies it — its salt, which `docs/ir-spec.md` settles is the value
    /// rather than a dense index beside it.
    ///
    /// A per-source uniform and not an element attribute, which is the one thing
    /// about it a reader is most likely to have backwards: the spec's table called
    /// it "implicit, carried, never declared" and the engine reached the other
    /// answer. A Set instantiates its chain per source, so an instance knows
    /// *statically* which geometry it runs over — and a value that is the same for
    /// every element an instance will ever touch is a uniform by definition.
    /// Carrying it on the element instead would be a `u32` on every element of
    /// every merged Set spent restating a constant.
    ///
    /// So nothing new is written to read it: `Source::salt` is per geometry,
    /// `Set::prepare` already writes it as `seed_salt` into every node's uniform
    /// block, and every generated module already declares the field. The read is
    /// the whole of what was missing.
    ///
    /// It is what a downstream mask compares against, and what it compares *to* is
    /// a [`SlotTy::Source`] slot — a `.kir` may not name a node, so the comparand
    /// arrives through a slot the Set binds.
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
            // **Three kinds now, and the third is an L5.** A frame pass is
            // handed a picture rather than the material that made it: there is
            // no element behind a texel, and the frame in front of it may hold
            // several decks' material at once.
            Ambient::Seed => !matches!(kind, Kind::L3 | Kind::Field | Kind::L5),
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
            // **And the same third kind**, for the sentence one line up: an L5
            // runs over a frame rather than over a geometry, and the frame it
            // is handed may hold several sources' material folded together — so
            // there is no chain instance for it to name.
            Ambient::Source => !matches!(kind, Kind::L3 | Kind::Field | Kind::L5),
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
            // **An L5 gets it too**, and not on the state argument above: a
            // frame effect that moves at a rate rather than to a position reads
            // the step it is being asked to advance by, and `dt` comes from a
            // record rather than from a clock so a replay pays nothing for it.
            Ambient::Dt => matches!(kind, Kind::L1 | Kind::L3 | Kind::L5),
            Ambient::Camera => kind == Kind::L4,
            // **The coordinate `tap` takes**, in a `frame` block, and it is the
            // same sentence it already means in a fragment one: 0..1 across the
            // primitive, which for a fullscreen pass is 0..1 across the frame.
            Ambient::PointCoord => matches!(block, BlockKind::Fragment | BlockKind::Frame),
            // Fragment-only, and not because a vertex stage could not be given
            // them: a fullscreen procedure has no vertex block at all, and in a
            // per-element one a ray through the fragment is not a thing a
            // vertex has. Offering them where they mean nothing would be one
            // more way to write a procedure that compiles and is wrong.
            Ambient::Eye | Ambient::Ray => kind == Kind::L4 && block == BlockKind::Fragment,
        }
    }
}
