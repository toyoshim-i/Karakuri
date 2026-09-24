#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Geometry generation. Stateful, compute.
    L1,
    /// Geometry modulation: `Geometry -> Geometry`. Stateless by rule, compute. See
    /// `docs/ir-spec.md`, "L2 and L3".
    L2,
    /// The camera: `() -> Camera`, producing camera viewpoint state for rendering.
    L3,
    /// Rendering. Stateless, a render pipeline.
    L4,
    /// A spatial function: `vec3 -> float`, spliced into caller procedures.
    Field,
    /// A frame effect: `[Texture] -> Texture`, executed in fullscreen fragment pass.
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
    Lines,
    /// Fullscreen pass without per-element geometry.
    Fullscreen,
}

impl Kind {
    /// All pipeline layer kinds supported by the runtime.
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

/// Incoming texture identifier passed to L5 procedures (`src`).
pub const TEXTURE_SRC: &str = "src";

/// Retained frame texture identifier passed to L5 procedures (`held`).
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

/// Blend mode for rasterized fragments in L4 rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Additive blending: color output adds to the framebuffer without occlusion.
    Additive,
    /// Weighted blended order-independent transparency (WBOIT).
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

    /// Returns the number of `f32` components if this type is permitted for a `param`
    /// (`float`, `vec2`, `vec3`), or `None` otherwise.
    pub fn param_components(self) -> Option<usize> {
        Some(match self {
            Ty::Float => 1,
            Ty::Vec2 => 2,
            Ty::Vec3 => 3,
            _ => return None,
        })
    }
}

/// Input dependency type bound to a `uses` slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotTy {
    /// The elements of an L1, read beside the ones this node runs over.
    Geometry,
    /// A `kind Field` procedure, evaluated at a point via function call syntax (`<slot>(p)`).
    Field,
    /// A viewpoint this renderer draws from (L3 procedure or built-in orbit).
    Camera,
    /// Identity (u32) of one geometry for comparison against [`Ambient::Source`].
    Source,
    /// A texture input folded into an L5 procedure.
    Texture,
}

impl SlotTy {
    /// Parses a slot type from its header identifier.
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

    /// Returns the header identifier corresponding to this slot type.
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
