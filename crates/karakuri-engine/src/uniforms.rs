//! Packing CPU values into a generated uniform layout.
//!
//! The generator computes each field's byte offset from WGSL's own alignment
//! rules and publishes the result as a [`UniformLayout`]. The engine writes
//! into that layout by field name rather than mirroring it as a `#[repr(C)]`
//! struct, because a mirrored struct is a second copy of the layout that
//! nothing checks: it compiles fine when it drifts, and the symptom is a
//! shader reading a parameter out of the middle of another one.
//!
//! Every named field must be written before upload. A field left unset is a
//! bug — the engine knows what the procedure declared — so this refuses to
//! produce bytes rather than uploading a silent zero.

use std::collections::HashSet;

use karakuri_codegen::layout::UniformLayout;

pub struct UniformPacker<'a> {
    layout: &'a UniformLayout,
    bytes: Vec<u8>,
    written: HashSet<String>,
}

impl<'a> UniformPacker<'a> {
    pub fn new(layout: &'a UniformLayout) -> UniformPacker<'a> {
        UniformPacker {
            layout,
            bytes: vec![0; layout.total_size as usize],
            written: HashSet::new(),
        }
    }

    fn write(&mut self, name: &str, wgsl_ty: &str, data: &[u8]) {
        let field = self
            .layout
            .fields
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("uniform layout has no field `{name}`"));
        assert_eq!(
            field.wgsl_ty, wgsl_ty,
            "field `{name}` is {} in the layout, not {wgsl_ty}",
            field.wgsl_ty
        );
        let at = field.offset as usize;
        self.bytes[at..at + data.len()].copy_from_slice(data);
        self.written.insert(name.to_string());
    }

    pub fn f32(&mut self, name: &str, v: f32) -> &mut Self {
        self.write(name, "f32", &v.to_le_bytes());
        self
    }

    pub fn u32(&mut self, name: &str, v: u32) -> &mut Self {
        self.write(name, "u32", &v.to_le_bytes());
        self
    }

    pub fn vec2(&mut self, name: &str, v: [f32; 2]) -> &mut Self {
        let mut b = [0u8; 8];
        for (i, x) in v.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&x.to_le_bytes());
        }
        self.write(name, "vec2<f32>", &b);
        self
    }

    pub fn vec3(&mut self, name: &str, v: [f32; 3]) -> &mut Self {
        // 12 bytes written into a slot aligned to 16: the generator already
        // placed the following field past the padding.
        let mut b = [0u8; 12];
        for (i, x) in v.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&x.to_le_bytes());
        }
        self.write(name, "vec3<f32>", &b);
        self
    }

    pub fn mat4(&mut self, name: &str, m: [[f32; 4]; 4]) -> &mut Self {
        let mut b = [0u8; 64];
        for (c, col) in m.iter().enumerate() {
            for (r, x) in col.iter().enumerate() {
                let at = (c * 4 + r) * 4;
                b[at..at + 4].copy_from_slice(&x.to_le_bytes());
            }
        }
        self.write(name, "mat4x4<f32>", &b);
        self
    }

    /// The packed bytes, once every named field has been written.
    ///
    /// Panics rather than uploading a partially filled buffer: a missing field
    /// reads as zero in the shader, and a parameter that is silently zero is
    /// far harder to find than a panic naming it.
    pub fn finish(self) -> Vec<u8> {
        let missing: Vec<&str> = self
            .layout
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .filter(|n| !self.written.contains(*n))
            .collect();
        assert!(
            missing.is_empty(),
            "uniform fields never written: {}",
            missing.join(", ")
        );
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use karakuri_codegen::layout::UniformLayoutBuilder;

    fn layout() -> UniformLayout {
        let mut b = UniformLayoutBuilder::new();
        b.field("t", "f32");
        b.field("capacity", "u32");
        b.field("glow", "vec3<f32>");
        b.finish().0
    }

    #[test]
    fn fields_land_at_the_offsets_the_generator_computed() {
        let layout = layout();
        let mut p = UniformPacker::new(&layout);
        p.f32("t", 1.5).u32("capacity", 7).vec3("glow", [1.0, 2.0, 3.0]);
        let bytes = p.finish();

        assert_eq!(bytes.len(), layout.total_size as usize);
        assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), 1.5);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 7);
        // vec3 aligns to 16, so it cannot follow the u32 at byte 8.
        let at = layout.fields[2].offset as usize;
        assert_eq!(at, 16);
        assert_eq!(f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()), 1.0);
    }

    #[test]
    #[should_panic(expected = "never written")]
    fn a_field_left_unset_is_refused_rather_than_uploaded_as_zero() {
        let layout = layout();
        let mut p = UniformPacker::new(&layout);
        p.f32("t", 1.0);
        let _ = p.finish();
    }

    #[test]
    #[should_panic(expected = "no field")]
    fn writing_a_field_the_layout_does_not_have_is_a_bug_not_a_no_op() {
        let layout = layout();
        let mut p = UniformPacker::new(&layout);
        p.f32("nonexistent", 1.0);
    }

    #[test]
    #[should_panic(expected = "not f32")]
    fn writing_a_field_at_the_wrong_type_is_refused() {
        let layout = layout();
        let mut p = UniformPacker::new(&layout);
        p.f32("capacity", 1.0);
    }
}
