//! `karakuri_ir::Ty` to WGSL type spelling.

use karakuri_ir::Ty;

/// Returns the WGSL type name corresponding to an IR type `Ty`.
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
