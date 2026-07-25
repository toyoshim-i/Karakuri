//! `karakuri_ir::Ty` to WGSL type spelling.

use karakuri_ir::Ty;

/// The WGSL spelling of an IR type. Used both for local variable casts
/// (`Construct` lowers to one of these as a constructor) and for storage
/// buffer element types on the L4 side, where a consumed attribute's natural
/// width (not the padded `vec4` it is stored as) is what a local variable
/// gets bound to.
pub fn wgsl_ty(ty: Ty) -> &'static str {
    match ty {
        Ty::Float => "f32",
        Ty::Int => "i32",
        Ty::Uint => "u32",
        Ty::Bool => "bool",
        Ty::Vec2 => "vec2<f32>",
        Ty::Vec3 => "vec3<f32>",
        Ty::Vec4 => "vec4<f32>",
        Ty::Mat3 => "mat3x3<f32>",
        Ty::Mat4 => "mat4x4<f32>",
    }
}

/// The swizzle that reads an attribute's natural width out of its padded
/// `vec4` storage slot.
pub fn attr_swizzle(ty: Ty) -> &'static str {
    match ty {
        Ty::Float => "x",
        Ty::Vec2 => "xy",
        Ty::Vec3 => "xyz",
        other => unreachable!("attribute type {other:?} is never float, vec2, or vec3"),
    }
}

/// Wraps a value expression of type `ty` into a `vec4` constructor matching
/// the padded storage layout every attribute buffer uses. `elem` is the WGSL
/// spelling of the scalar element type (`"f32"` or `"u32"`).
pub fn pad_to_vec4(ty: Ty, elem: &str, value: &str) -> String {
    let zero = if elem == "u32" { "0u" } else { "0.0" };
    match ty {
        Ty::Float | Ty::Uint => format!("vec4<{elem}>({value}, {zero}, {zero}, {zero})"),
        Ty::Vec2 => format!("vec4<{elem}>({value}, {zero}, {zero})"),
        Ty::Vec3 => format!("vec4<{elem}>({value}, {zero})"),
        other => unreachable!("attribute type {other:?} is never float, vec2, vec3, or uint"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swizzle_widths_match_component_counts() {
        assert_eq!(attr_swizzle(Ty::Float).len(), 1);
        assert_eq!(attr_swizzle(Ty::Vec2).len(), 2);
        assert_eq!(attr_swizzle(Ty::Vec3).len(), 3);
    }
}
