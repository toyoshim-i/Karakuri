use super::{BlockKind, Kind, Ty};

/// Attributes emitted by an L1 procedure and consumed by downstream procedures.
///
/// Note that implicit system attributes like `seed` are represented in [`Ambient`].
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

    /// Returns the derivation rule for synthesizing this attribute if un-emitted upstream.
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

/// Rule describing how an un-emitted attribute is synthesized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Derivation {
    /// `age` derived from current clock time minus spawn timestamp (`birth_t`).
    SinceBirth,
    /// `velocity` derived from the change in a source attribute divided by frame `dt`.
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

    /// Returns true if the engine stores this derived attribute in the element buffer
    /// rather than synthesizing it on demand at the read site.
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
    /// vertex: a segment's far end in clip space for [`Topology::Lines`].
    ClipB,
    /// vertex: point sprite size or line stroke width as a fraction of target height.
    PointRate,
    /// fragment: linear RGB, straight alpha.
    Color,
    /// camera: world-space position. Required in camera block.
    Eye,
    /// camera: world-space look-at target. Required in camera block.
    Target,
    /// camera: world-space up vector. Defaults to `+y`.
    Up,
    /// camera: vertical field of view in radians. Defaults to `pi / 3`.
    FovY,
    /// camera: near clipping plane distance. Defaults to 0.1.
    Near,
    /// mask: deformation modulation factor in `[0, 1]`. Required in mask block.
    Strength,
    /// field: signed distance to surface at [`Ambient::Point`]. Required in field block.
    Distance,
    /// camera: far clipping plane distance. Defaults to 100.
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

/// Ambient values accessible without explicit parameter declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ambient {
    /// Carried per-element random seed.
    Seed,
    /// Copy index of the current element produced by an upstream amplifying L2 node (`0` if unamplified).
    Copy,
    /// World-space evaluation point in field procedures.
    Point,
    Capacity,
    T,
    /// Musical position in beats on the session tempo grid.
    Beats,
    Dt,
    /// View-projection matrix in L4 procedures.
    Camera,
    PointCoord,
    /// World-space camera eye position in L4 fragment shaders.
    Eye,
    /// World-space unit ray direction through the current fragment in L4 fragment shaders.
    Ray,
    /// Identifier / salt of the geometry source running this chain instance.
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
            // Neither L3, Field, nor L5 have elements or associated geometry salt.
            Ambient::Seed => !matches!(kind, Kind::L3 | Kind::Field | Kind::L5),
            // Source uniform is valid across all geometry instances, but absent in L3, Field, and L5.
            Ambient::Source => !matches!(kind, Kind::L3 | Kind::Field | Kind::L5),
            // Copy index is valid in L2 deforms and downstream L4 rendering.
            Ambient::Copy => kind == Kind::L2 || kind == Kind::L4,
            // Point is only valid within field procedures.
            Ambient::Point => block == BlockKind::Field,
            Ambient::T | Ambient::Beats => true,
            Ambient::Capacity => kind == Kind::L1,
            // Simulation, camera, and post-process stages receive time step dt.
            Ambient::Dt => matches!(kind, Kind::L1 | Kind::L3 | Kind::L5),
            Ambient::Camera => kind == Kind::L4,
            // Point coordination across fragment and post-process frames.
            Ambient::PointCoord => matches!(block, BlockKind::Fragment | BlockKind::Frame),
            // World-space ray inputs are only available in L4 fragment shaders.
            Ambient::Eye | Ambient::Ray => kind == Kind::L4 && block == BlockKind::Fragment,
        }
    }
}
