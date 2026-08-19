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
