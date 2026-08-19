//! The builtin function table.
//!
//! Three passes need this and they need it to agree: the check pass types
//! calls against it, cost estimation weighs them, and code generation lowers
//! them. A builtin whose signature is described in three places will eventually
//! be described three different ways, so it is described here.
//!
//! Signatures are described rather than enumerated. Most of these functions are
//! componentwise over `float` and the vector widths — `abs` is four overloads
//! written once — so a signature names a shape and the domain that shape ranges
//! over, and the check pass unifies. See `docs/ir-spec.md`, Built-in functions.

use crate::ast::Ty;

/// What a single argument or result position accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A fixed type, regardless of what the other positions resolve to.
    Exact(Ty),
    /// The generic type this call resolves to. Every `Same` position in one
    /// call is the same type.
    Same,
    /// `float`, whatever the generic type is. `length` and `dot` are this:
    /// vector in, scalar out.
    Scalar,
}

/// Which types the generic position may resolve to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// No generic position; the signature is fully concrete.
    None,
    /// `float`, `vec2`, `vec3`, `vec4`.
    FloatOrVector,
    /// Vectors only. `normalize` on a scalar is meaningless.
    VectorOnly,
}

#[derive(Debug, Clone, Copy)]
pub struct Signature {
    pub args: &'static [Shape],
    pub ret: Shape,
    pub domain: Domain,
    /// Argument positions that must be compile-time constants. `fbm`'s octave
    /// count is the only one: it is unrolled at lowering time, so it cannot be
    /// a runtime value.
    pub const_args: &'static [usize],
}

macro_rules! builtins {
    ($( $variant:ident => $name:literal, [$($arg:expr),*] -> $ret:expr, $domain:expr, [$($ca:literal),*] );* $(;)?) => {
        /// Every function callable from IR. A name that is not here is not a
        /// builtin; the check pass then tries it as a type constructor, and
        /// failing that reports it undefined.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Builtin { $($variant),* }

        impl Builtin {
            pub const ALL: &'static [Builtin] = &[$(Builtin::$variant),*];

            pub fn name(self) -> &'static str {
                match self { $(Builtin::$variant => $name),* }
            }

            pub fn from_name(name: &str) -> Option<Builtin> {
                match name { $($name => Some(Builtin::$variant),)* _ => None }
            }

            pub fn signature(self) -> Signature {
                match self {
                    $(Builtin::$variant => Signature {
                        args: &[$($arg),*],
                        ret: $ret,
                        domain: $domain,
                        const_args: &[$($ca),*],
                    }),*
                }
            }
        }
    };
}

use Domain::{FloatOrVector, None as Concrete, VectorOnly};
use Shape::{Exact, Same, Scalar};
use Ty::{Float, Int, Uint, Vec2, Vec3};

builtins! {
    // Math
    Abs        => "abs",        [Same] -> Same, FloatOrVector, [];
    Floor      => "floor",      [Same] -> Same, FloatOrVector, [];
    Ceil       => "ceil",       [Same] -> Same, FloatOrVector, [];
    Round      => "round",      [Same] -> Same, FloatOrVector, [];
    Fract      => "fract",      [Same] -> Same, FloatOrVector, [];
    Mod        => "mod",        [Same, Same] -> Same, FloatOrVector, [];
    Min        => "min",        [Same, Same] -> Same, FloatOrVector, [];
    Max        => "max",        [Same, Same] -> Same, FloatOrVector, [];
    Clamp      => "clamp",      [Same, Same, Same] -> Same, FloatOrVector, [];
    Mix        => "mix",        [Same, Same, Same] -> Same, FloatOrVector, [];
    Step       => "step",       [Same, Same] -> Same, FloatOrVector, [];
    Smoothstep => "smoothstep", [Same, Same, Same] -> Same, FloatOrVector, [];
    Sign       => "sign",       [Same] -> Same, FloatOrVector, [];
    Sqrt       => "sqrt",       [Same] -> Same, FloatOrVector, [];
    Pow        => "pow",        [Same, Same] -> Same, FloatOrVector, [];
    Exp        => "exp",        [Same] -> Same, FloatOrVector, [];
    Log        => "log",        [Same] -> Same, FloatOrVector, [];
    Exp2       => "exp2",       [Same] -> Same, FloatOrVector, [];
    Log2       => "log2",       [Same] -> Same, FloatOrVector, [];

    // Trigonometry
    Sin   => "sin",   [Same] -> Same, FloatOrVector, [];
    Cos   => "cos",   [Same] -> Same, FloatOrVector, [];
    Tan   => "tan",   [Same] -> Same, FloatOrVector, [];
    Asin  => "asin",  [Same] -> Same, FloatOrVector, [];
    Acos  => "acos",  [Same] -> Same, FloatOrVector, [];
    Atan  => "atan",  [Same] -> Same, FloatOrVector, [];
    Atan2 => "atan2", [Same, Same] -> Same, FloatOrVector, [];

    // Vector
    Length    => "length",    [Same] -> Scalar, VectorOnly, [];
    Distance  => "distance",  [Same, Same] -> Scalar, VectorOnly, [];
    Normalize => "normalize", [Same] -> Same, VectorOnly, [];
    Dot       => "dot",       [Same, Same] -> Scalar, VectorOnly, [];
    Cross     => "cross",     [Exact(Vec3), Exact(Vec3)] -> Exact(Vec3), Concrete, [];
    Reflect   => "reflect",   [Same, Same] -> Same, VectorOnly, [];
    Refract   => "refract",   [Same, Same, Exact(Float)] -> Same, VectorOnly, [];

    // Hash and noise. Salted per *source* — every layer over one geometry
    // shares the salt, and two geometries in one Set get different ones — so
    // re-seeding a Set changes randomness without touching structure, and two
    // sources built from the same procedure do not draw the same dust.
    Hash1      => "hash1",       [Exact(Uint)] -> Exact(Float), Concrete, [];
    Hash2      => "hash2",       [Exact(Uint)] -> Exact(Vec2), Concrete, [];
    Hash3      => "hash3",       [Exact(Uint)] -> Exact(Vec3), Concrete, [];
    ValueNoise => "value_noise", [Exact(Vec3)] -> Exact(Float), Concrete, [];
    Perlin     => "perlin",      [Exact(Vec3)] -> Exact(Float), Concrete, [];
    Simplex    => "simplex",     [Exact(Vec3)] -> Exact(Float), Concrete, [];
    Fbm        => "fbm",         [Exact(Vec3), Exact(Int)] -> Exact(Float), Concrete, [1];
    Curl       => "curl",        [Exact(Vec3)] -> Exact(Vec3), Concrete, [];

    // SDF
    SdSphere      => "sd_sphere",       [Exact(Vec3), Exact(Float)] -> Exact(Float), Concrete, [];
    SdBox         => "sd_box",          [Exact(Vec3), Exact(Vec3)] -> Exact(Float), Concrete, [];
    SdTorus       => "sd_torus",        [Exact(Vec3), Exact(Vec2)] -> Exact(Float), Concrete, [];
    SdPlane       => "sd_plane",        [Exact(Vec3), Exact(Vec3), Exact(Float)] -> Exact(Float), Concrete, [];
    OpUnion        => "op_union",        [Exact(Float), Exact(Float)] -> Exact(Float), Concrete, [];
    OpSmoothUnion  => "op_smooth_union", [Exact(Float), Exact(Float), Exact(Float)] -> Exact(Float), Concrete, [];
    OpSubtract     => "op_subtract",     [Exact(Float), Exact(Float)] -> Exact(Float), Concrete, [];
    OpIntersect    => "op_intersect",    [Exact(Float), Exact(Float)] -> Exact(Float), Concrete, [];

    // **The Set's field, evaluated at a point.** The one builtin whose body is
    // not in this compiler: it lowers to a call of the function a `kind Field`
    // procedure was spliced in as, so a Set that holds no field cannot satisfy
    // a procedure that calls this — refused where the Set is built, which is
    // the first point holding both.
    //
    // Its *weight* is not here either, and cannot be: what one evaluation
    // costs is the field's own `ops_per_evaluation`, which belongs to another
    // file. Cost estimation counts the call sites instead and the Set does the
    // multiplication. See `Cost::field_calls`.
    Field => "field", [Exact(Vec3)] -> Exact(Float), Concrete, [];

    // Transform
    RotX    => "rot_x",    [Exact(Vec3), Exact(Float)] -> Exact(Vec3), Concrete, [];
    RotY    => "rot_y",    [Exact(Vec3), Exact(Float)] -> Exact(Vec3), Concrete, [];
    RotZ    => "rot_z",    [Exact(Vec3), Exact(Float)] -> Exact(Vec3), Concrete, [];
    RotAxis => "rot_axis", [Exact(Vec3), Exact(Vec3), Exact(Float)] -> Exact(Vec3), Concrete, [];

    // Distribution
    SpherePoint => "sphere_point", [Exact(Float), Exact(Float)] -> Exact(Vec3), Concrete, [];
    DiscPoint   => "disc_point",   [Exact(Float), Exact(Float)] -> Exact(Vec2), Concrete, [];

    // Colour. `hsv_to_rgb` returns linear RGB: it converts internally so that
    // authors get the hue they expect without thinking about colour space.
    HsvToRgb     => "hsv_to_rgb",     [Exact(Vec3)] -> Exact(Vec3), Concrete, [];
    RgbToHsv     => "rgb_to_hsv",     [Exact(Vec3)] -> Exact(Vec3), Concrete, [];
    SrgbToLinear => "srgb_to_linear", [Exact(Vec3)] -> Exact(Vec3), Concrete, [];
    LinearToSrgb => "linear_to_srgb", [Exact(Vec3)] -> Exact(Vec3), Concrete, [];
}

impl Builtin {
    /// Whether this builtin reads the seed stream salt, and so needs it in
    /// scope wherever it is lowered.
    pub fn is_seeded(self) -> bool {
        matches!(
            self,
            Builtin::Hash1
                | Builtin::Hash2
                | Builtin::Hash3
                | Builtin::ValueNoise
                | Builtin::Perlin
                | Builtin::Simplex
                | Builtin::Fbm
                | Builtin::Curl
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for b in Builtin::ALL {
            assert_eq!(Builtin::from_name(b.name()), Some(*b), "{}", b.name());
        }
    }

    #[test]
    fn there_is_no_id_producing_builtin() {
        assert!(Builtin::from_name("id").is_none());
    }

    #[test]
    fn generic_signatures_declare_a_domain_and_concrete_ones_do_not() {
        // A `Same` position with no domain would have nothing to unify over,
        // and a domain with no `Same` position would never be consulted.
        for b in Builtin::ALL {
            let sig = b.signature();
            let generic = sig.args.iter().chain([&sig.ret]).any(|s| {
                matches!(s, Shape::Same | Shape::Scalar)
            });
            assert_eq!(
                generic,
                sig.domain != Domain::None,
                "{} disagrees about being generic",
                b.name()
            );
        }
    }

    #[test]
    fn only_fbm_takes_a_constant_argument() {
        // Octaves are unrolled at lowering time, so they cannot be a runtime
        // value. Nothing else has that property.
        let with_consts: Vec<_> = Builtin::ALL
            .iter()
            .filter(|b| !b.signature().const_args.is_empty())
            .collect();
        assert_eq!(with_consts, vec![&Builtin::Fbm]);
    }

    #[test]
    fn const_argument_positions_are_in_range() {
        for b in Builtin::ALL {
            let sig = b.signature();
            for &i in sig.const_args {
                assert!(i < sig.args.len(), "{} has a stray const_arg", b.name());
            }
        }
    }
}
