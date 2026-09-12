//! Packing CPU parameter values into uniform buffer layouts.
//!
//! Provides [`UniformScratch`] and [`UniformPacker`] to validate and serialize
//! named uniform fields into GPU-ready byte representations without per-frame heap allocations.

use karakuri_codegen::layout::UniformLayout;

/// Reusable host-side staging buffer for packing uniform values without per-frame allocations.
pub struct UniformScratch {
    bytes: Vec<u8>,
    /// Tracks whether each field index in the layout has been written.
    written: Vec<bool>,
}

impl UniformScratch {
    pub fn new(layout: &UniformLayout) -> UniformScratch {
        UniformScratch {
            bytes: vec![0; layout.total_size as usize],
            written: vec![false; layout.fields.len()],
        }
    }

    /// Clears scratch storage in place and returns a packer for writing frame uniforms.
    pub fn pack<'a>(&'a mut self, layout: &'a UniformLayout) -> UniformPacker<'a> {
        assert_eq!(
            self.bytes.len(),
            layout.total_size as usize,
            "scratch was sized against a different uniform layout"
        );
        assert_eq!(
            self.written.len(),
            layout.fields.len(),
            "scratch was sized against a different uniform layout"
        );
        self.bytes.fill(0);
        self.written.fill(false);
        UniformPacker {
            layout,
            scratch: self,
        }
    }
}

/// Serializes and validates field values against a [`UniformLayout`].
pub struct UniformPacker<'a> {
    layout: &'a UniformLayout,
    scratch: &'a mut UniformScratch,
}

impl<'a> UniformPacker<'a> {
    fn write(&mut self, name: &str, wgsl_ty: &str, data: &[u8]) {
        let layout = self.layout;
        let (index, field) = layout
            .fields
            .iter()
            .enumerate()
            .find(|(_, f)| f.name == name)
            .unwrap_or_else(|| panic!("uniform layout has no field `{name}`"));
        assert_eq!(
            field.wgsl_ty, wgsl_ty,
            "field `{name}` is {} in the layout, not {wgsl_ty}",
            field.wgsl_ty
        );
        let at = field.offset as usize;
        self.scratch.bytes[at..at + data.len()].copy_from_slice(data);
        self.scratch.written[index] = true;
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
        // 12-byte vec3 aligned to 16 bytes per WGSL uniform rules.
        let mut b = [0u8; 12];
        for (i, x) in v.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&x.to_le_bytes());
        }
        self.write(name, "vec3<f32>", &b);
        self
    }

    pub fn vec4(&mut self, name: &str, v: [f32; 4]) -> &mut Self {
        let mut b = [0u8; 16];
        for (i, x) in v.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&x.to_le_bytes());
        }
        self.write(name, "vec4<f32>", &b);
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

    /// Finalizes packing and returns a byte slice of the serialized uniform data.
    ///
    /// # Panics
    ///
    /// Panics if any field defined in the [`UniformLayout`] was not written.
    pub fn finish(self) -> &'a [u8] {
        if self.scratch.written.iter().any(|w| !w) {
            let missing: Vec<&str> = self
                .layout
                .fields
                .iter()
                .zip(&self.scratch.written)
                .filter(|(_, written)| !**written)
                .map(|(f, _)| f.name.as_str())
                .collect();
            panic!("uniform fields never written: {}", missing.join(", "));
        }
        &self.scratch.bytes
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
        let mut scratch = UniformScratch::new(&layout);
        let mut p = scratch.pack(&layout);
        p.f32("t", 1.5)
            .u32("capacity", 7)
            .vec3("glow", [1.0, 2.0, 3.0]);
        let bytes = p.finish();

        assert_eq!(bytes.len(), layout.total_size as usize);
        assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), 1.5);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 7);
        // vec3 aligns to 16, so it cannot follow the u32 at byte 8.
        let at = layout.fields[2].offset as usize;
        assert_eq!(at, 16);
        assert_eq!(
            f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()),
            1.0
        );
    }

    /// Verifies that consecutive pack operations overwrite all fields without carryover.
    #[test]
    fn a_reused_scratch_carries_nothing_over_from_the_previous_frame() {
        let layout = layout();
        let mut scratch = UniformScratch::new(&layout);

        let mut p = scratch.pack(&layout);
        p.f32("t", 1.5)
            .u32("capacity", 7)
            .vec3("glow", [1.0, 2.0, 3.0]);
        let first: Vec<u8> = p.finish().to_vec();

        let mut p = scratch.pack(&layout);
        p.f32("t", 2.5)
            .u32("capacity", 9)
            .vec3("glow", [4.0, 5.0, 6.0]);
        let second = p.finish();

        assert_ne!(first.as_slice(), second);
        assert_eq!(f32::from_le_bytes(second[0..4].try_into().unwrap()), 2.5);
        assert_eq!(u32::from_le_bytes(second[4..8].try_into().unwrap()), 9);
        let at = layout.fields[2].offset as usize;
        assert_eq!(
            f32::from_le_bytes(second[at..at + 4].try_into().unwrap()),
            4.0
        );
    }

    /// Verifies that field write tracking resets across pack invocations.
    #[test]
    #[should_panic(expected = "never written")]
    fn a_reused_scratch_does_not_remember_that_a_field_was_written_last_frame() {
        let layout = layout();
        let mut scratch = UniformScratch::new(&layout);

        let mut p = scratch.pack(&layout);
        p.f32("t", 1.5)
            .u32("capacity", 7)
            .vec3("glow", [1.0, 2.0, 3.0]);
        let _ = p.finish();

        let mut p = scratch.pack(&layout);
        p.f32("t", 2.5).u32("capacity", 9);
        let _ = p.finish();
    }

    #[test]
    #[should_panic(expected = "never written")]
    fn a_field_left_unset_is_refused_rather_than_uploaded_as_zero() {
        let layout = layout();
        let mut scratch = UniformScratch::new(&layout);
        let mut p = scratch.pack(&layout);
        p.f32("t", 1.0);
        let _ = p.finish();
    }

    #[test]
    #[should_panic(expected = "no field")]
    fn writing_a_field_the_layout_does_not_have_is_a_bug_not_a_no_op() {
        let layout = layout();
        let mut scratch = UniformScratch::new(&layout);
        let mut p = scratch.pack(&layout);
        p.f32("nonexistent", 1.0);
    }

    /// Verifies that packing against an mismatched layout panics.
    #[test]
    #[should_panic(expected = "sized against a different uniform layout")]
    fn a_scratch_cannot_be_packed_against_a_layout_it_was_not_sized_for() {
        let mine = layout();
        let mut scratch = UniformScratch::new(&mine);

        let mut b = UniformLayoutBuilder::new();
        b.field("t", "f32");
        b.field("capacity", "u32");
        b.field("glow", "vec3<f32>");
        b.field("extra", "mat4x4<f32>");
        let theirs = b.finish().0;

        let _ = scratch.pack(&theirs);
    }

    #[test]
    #[should_panic(expected = "not f32")]
    fn writing_a_field_at_the_wrong_type_is_refused() {
        let layout = layout();
        let mut scratch = UniformScratch::new(&layout);
        let mut p = scratch.pack(&layout);
        p.f32("capacity", 1.0);
    }
}
