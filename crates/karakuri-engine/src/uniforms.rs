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
//!
//! ## Why the storage is owned by the caller
//!
//! Packing happens in `Set::prepare`, which runs once per frame on the render
//! thread, where the first invariant in `README.md` is "never allocate". An
//! earlier version of this built a fresh `Vec<u8>` and a fresh
//! `HashSet<String>` per call and pushed a freshly allocated `String` into
//! that set for every field it wrote — three kinds of allocation per uniform
//! buffer per frame, two uniform buffers per frame. The layout is fixed at
//! build time, so all of it is knowable then: a [`UniformScratch`] sized once
//! against the layout is handed back to a [`UniformPacker`] every frame,
//! cleared in place, and written into. Nothing here allocates after
//! construction, and `finish` hands back a borrow of the scratch rather than
//! a `Vec` it had to build.

use karakuri_codegen::layout::UniformLayout;

/// Reusable storage for one uniform buffer's worth of packing, sized once
/// against the layout it will be written through.
///
/// Owned by whatever writes that buffer every frame — `Set` holds one per
/// uniform group — and borrowed for the duration of a single frame's writes
/// by [`UniformScratch::pack`].
pub struct UniformScratch {
    bytes: Vec<u8>,
    /// One flag per layout field, by field index. A `HashSet<String>` said the
    /// same thing and allocated a `String` per field per frame to say it; the
    /// layout's field order is fixed at build time, so an index is enough.
    written: Vec<bool>,
}

impl UniformScratch {
    pub fn new(layout: &UniformLayout) -> UniformScratch {
        UniformScratch {
            bytes: vec![0; layout.total_size as usize],
            written: vec![false; layout.fields.len()],
        }
    }

    /// Borrow this scratch for one frame's writes.
    ///
    /// Clears in place: `fill` over storage that is already the right size,
    /// so there is nothing to allocate, nothing to grow, and nothing to drop.
    /// `layout` must be the one this scratch was sized against — passing a
    /// different one is a bug rather than a resize, since the pipelines built
    /// against the original are still the ones being fed.
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

pub struct UniformPacker<'a> {
    layout: &'a UniformLayout,
    scratch: &'a mut UniformScratch,
}

impl<'a> UniformPacker<'a> {
    fn write(&mut self, name: &str, wgsl_ty: &str, data: &[u8]) {
        // Copied out of `self` first: `self.layout` is a shared reference with
        // the scratch's lifetime, and reading it *through* `self` would keep a
        // borrow of `self` alive across the mutation of `self.scratch` below.
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
    ///
    /// The check is a scan for *any* unwritten flag before the list of names
    /// is built, so the successful path — every frame — collects nothing and
    /// allocates nothing. The returned slice borrows the scratch, so the
    /// caller uploads straight out of storage that outlives the frame.
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

    /// The whole point of reusing the storage: the second frame's bytes must
    /// be the second frame's, not the first frame's with some of them
    /// overwritten. A field the layout has but the caller happens to write
    /// the same value into every frame would hide a missing `fill`; writing
    /// different values and checking all of them is what catches it.
    #[test]
    fn a_reused_scratch_carries_nothing_over_from_the_previous_frame() {
        let layout = layout();
        let mut scratch = UniformScratch::new(&layout);

        let mut p = scratch.pack(&layout);
        p.f32("t", 1.5).u32("capacity", 7).vec3("glow", [1.0, 2.0, 3.0]);
        let first: Vec<u8> = p.finish().to_vec();

        let mut p = scratch.pack(&layout);
        p.f32("t", 2.5).u32("capacity", 9).vec3("glow", [4.0, 5.0, 6.0]);
        let second = p.finish();

        assert_ne!(first.as_slice(), second);
        assert_eq!(f32::from_le_bytes(second[0..4].try_into().unwrap()), 2.5);
        assert_eq!(u32::from_le_bytes(second[4..8].try_into().unwrap()), 9);
        let at = layout.fields[2].offset as usize;
        assert_eq!(f32::from_le_bytes(second[at..at + 4].try_into().unwrap()), 4.0);
    }

    /// And the bookkeeping resets with it: a field written on the first frame
    /// and forgotten on the second must still be refused. A `written` set that
    /// were merely reused rather than cleared would pass this silently.
    #[test]
    #[should_panic(expected = "never written")]
    fn a_reused_scratch_does_not_remember_that_a_field_was_written_last_frame() {
        let layout = layout();
        let mut scratch = UniformScratch::new(&layout);

        let mut p = scratch.pack(&layout);
        p.f32("t", 1.5).u32("capacity", 7).vec3("glow", [1.0, 2.0, 3.0]);
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

    #[test]
    #[should_panic(expected = "not f32")]
    fn writing_a_field_at_the_wrong_type_is_refused() {
        let layout = layout();
        let mut scratch = UniformScratch::new(&layout);
        let mut p = scratch.pack(&layout);
        p.f32("capacity", 1.0);
    }
}
