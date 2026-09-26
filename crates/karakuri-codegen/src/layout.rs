//! Pipeline binding layout specifications and buffer contracts across compute, render, and post passes.

use karakuri_ir::layout::{align_up, ElementLayout, StorageElemTy};

/// Every compute entry point (`spawn`, `element`) uses this workgroup size.
pub const WORKGROUP_SIZE: u32 = 64;

/// Vertices emitted per element by the L4 vertex stage. WebGPU has no point
/// size, so `topology points` expands to two triangles covering a quad
/// rather than to `PrimitiveTopology::PointList`.
pub const VERTICES_PER_ELEMENT: u32 = 6;

/// Bind group indices. Shared constants so L1 and L4 code (and the engine)
/// spell the same number the same way.
pub mod group {
    /// The uniform buffer, both stages. L1 additionally carries the engine's
    /// per-frame count state here — see the module doc.
    pub const UNIFORMS: u32 = 0;
    /// L1 only: the previous-frame element and alive buffers, read-only.
    pub const PREV: u32 = 1;
    /// L1 only: the next-frame element and alive buffers, read-write.
    pub const NEXT: u32 = 2;
    /// L1 `spawn` only: this substep's [`super::step_args`].
    pub const STEP: u32 = 3;
    /// L4 only: the current element and alive buffers (L1's post-swap `PREV`
    /// set), read-only. Reuses binding number 1 because a shader module is
    /// either L1 or L4, never both, so the numbers never collide in one
    /// pipeline.
    pub const ATTRS: u32 = 1;
    /// L3 only: the `CameraState` this procedure writes, read-write storage.
    /// Reuses number 1 on the same terms as [`ATTRS`] — a module is one kind
    /// of procedure, so the two can never both be bound.
    pub const STATE: u32 = 1;
}

/// Binding numbers, per group. [`group::PREV`], [`group::NEXT`] and
/// [`group::ATTRS`] use [`binding::ELEMENT`] and [`binding::ALIVE`];
/// [`group::UNIFORMS`] and [`group::STEP`] use the rest.
pub mod binding {
    /// `array<Element>`.
    pub const ELEMENT: u32 = 0;
    /// `array<u32>`, one flag per element. Not part of `Element` — see the
    /// module doc for why it is its own buffer.
    pub const ALIVE: u32 = 1;
    /// [`group::PREV`](super::group::PREV): the **far** geometry, for an L2
    /// that declares `uses <name> : Geometry` — `array<ElementFar>`, read-only.
    ///
    /// Binding index for far geometry input storage buffer in an L2 node.
    pub const FAR: u32 = 2;

    /// A uniform buffer: `Uniforms` in [`group::UNIFORMS`], `StepArgs` in
    /// [`group::STEP`](super::group::STEP).
    pub const UNIFORM: u32 = 0;
    /// [`group::UNIFORMS`]: the [`super::counts`] buffer, read-only storage.
    pub const COUNTS: u32 = 1;
    /// [`group::UNIFORMS`]: the scan's `array<u32>` destination indices,
    /// read-only storage. Compacted procedures only.
    pub const DEST: u32 = 2;
}

/// Buffer offsets and WGSL declarations for engine-state counters and indirect dispatch arguments.
pub mod counts {
    /// `dispatch_workgroups_indirect` arguments for `element`: three `u32`s.
    pub const ELEM_XYZ: u64 = 0;
    /// Entries of `prev` worth scanning and dispatching over. Not the alive
    /// count: some of those entries were killed during the previous step and
    /// are reclaimed by this step's scan.
    pub const RANGE: u64 = 12;
    /// `draw_indirect` arguments: `vertex_count`, `instance_count`,
    /// `first_vertex`, `first_instance`.
    pub const DRAW: u64 = 16;
    /// `S`, the survivor total the scan's `finalize` writes. Read by `spawn`
    /// to find the head of the free range, and by `advance` to roll `range`
    /// forward.
    pub const SURVIVORS: u64 = 32;
    /// Byte size of the buffer. The struct's own fields end at 36; the rest
    /// is padding so the size is a round 16-byte multiple.
    pub const SIZE: u64 = 48;

    /// The WGSL declaration. Emitted verbatim into both the generated L1
    /// source and the engine's own scan shader, so the two cannot drift.
    pub const WGSL: &str = "\
struct Counts {
    elem_x: u32,
    elem_y: u32,
    elem_z: u32,
    range: u32,
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
    survivors: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};
";
}

/// GPU camera representations on the GPU.
///
/// `CameraState` represents producer input (48 bytes, storage buffer).
/// `Camera` represents derived camera matrices/vectors for renderers (144 bytes, uniform buffer).
pub mod camera {
    /// Byte size of `CameraState`.
    pub const STATE_SIZE: u64 = 48;
    /// Byte size of `Camera`.
    pub const SIZE: u64 = 144;

    /// Host or compute shader camera output representation.
    ///
    /// Named `look_at` in WGSL because `target` is a reserved keyword in WGSL.
    pub const STATE_WGSL: &str = "\
struct CameraState {
    eye: vec3<f32>,
    fov_y: f32,
    look_at: vec3<f32>,
    near: f32,
    up: vec3<f32>,
    far: f32,
};
";

    /// WGSL definition of the Camera uniform struct.
    ///
    /// `right` and `up` are pre-scaled by FOV and aspect ratio.
    /// `depth_range` contains `(near, 1 / (far - near))`.
    pub const WGSL: &str = "\
struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec3<f32>,
    _pad0: f32,
    fwd: vec3<f32>,
    _pad1: f32,
    right: vec3<f32>,
    _pad2: f32,
    up: vec3<f32>,
    _pad3: f32,
    depth_range: vec2<f32>,
    _pad4: vec2<f32>,
};
";
}

/// Per-substep spawn parameters differing between substeps of a single frame.
pub mod step_args {
    /// Byte size of one entry.
    pub const SIZE: u64 = 20;
    /// Distance between consecutive substeps' entries, aligned to 256-byte WebGPU offset.
    pub const STRIDE: u64 = 256;

    /// The WGSL declaration shared with engine passes.
    pub const WGSL: &str = "\
struct StepArgs {
    spawn_count: u32,
    seed_base: u32,
    capacity: u32,
    t: f32,
    beats: f32,
};
";
}

/// Writes `struct Element { ... };` for `layout`.
pub fn write_element_struct(out: &mut String, layout: &ElementLayout) {
    write_element_struct_named(out, "Element", layout);
}

/// Writes a named element struct definition from an `ElementLayout`.
pub fn write_element_struct_named(out: &mut String, name: &str, layout: &ElementLayout) {
    out.push_str(&format!("struct {name} {{\n"));
    for s in &layout.slots {
        out.push_str(&format!("    {}: {},\n", s.name, s.elem_ty.wgsl_name()));
    }
    out.push_str("};\n");
}

/// Metadata describing one field in a generated uniform buffer struct.
#[derive(Debug, Clone, PartialEq)]
pub struct UniformField {
    /// Semantic identifier (e.g., parameter or engine variable name).
    pub name: String,
    /// Mangled WGSL field identifier.
    pub wgsl_name: String,
    /// WGSL type string (e.g. `"f32"`).
    pub wgsl_ty: &'static str,
    /// Byte offset within uniform buffer.
    pub offset: u32,
    /// Field byte size.
    pub size: u32,
}

/// Prefixes user parameter identifiers with `param_` to avoid WGSL keyword collisions.
pub fn mangle_param(name: &str) -> String {
    format!("param_{name}")
}

/// Returns the mangled WGSL field identifier for a spliced field's parameter.
///
/// Prefixes with `field_{slot}_{name}` to avoid collisions with caller uniform struct fields.
pub fn mangle_field_param(slot: &str, name: &str) -> String {
    format!("field_{slot}_{name}")
}

/// Returns the semantic lookup key for a field parameter used in host uniform packing.
///
/// Uses `\u{1}` control character as separator to prevent collisions with user-defined identifiers.
pub fn field_param_key(slot: &str, name: &str) -> String {
    format!("field\u{1}{slot}\u{1}{name}")
}

/// Returns the WGSL uniform identifier for a declared Source slot.
pub fn mangle_source_slot(slot: &str) -> String {
    format!("source_{slot}")
}

/// Returns the lookup key for a Source slot uniform field.
///
/// Uses an unrepresentable ASCII `\x01` separator to prevent collision with user identifiers.
pub fn source_slot_key(slot: &str) -> String {
    format!("source\u{1}{slot}")
}

/// Memory layout of a generated uniform struct, walked in order by the engine to pack data.
#[derive(Debug, Clone, PartialEq)]
pub struct UniformLayout {
    pub fields: Vec<UniformField>,
    /// Total uniform buffer size, aligned to a multiple of 16 bytes per WGSL rules.
    pub total_size: u32,
}

/// Returns WGSL uniform-address-space `(align, size)` for supported type strings.
///
/// Defers to [`StorageElemTy`] for types shared with storage elements, and defines
/// alignments for uniform-only types (`i32`, `vec4<f32>`, `mat4x4<f32>`).
pub fn align_size(wgsl_ty: &str) -> (u32, u32) {
    if let Some(elem) = StorageElemTy::from_wgsl_name(wgsl_ty) {
        return (elem.align(), elem.size());
    }
    match wgsl_ty {
        "i32" => (4, 4),
        "vec4<f32>" => (16, 16),
        "mat4x4<f32>" => (16, 64),
        other => panic!("align_size: unhandled uniform field type {other}"),
    }
}

/// Builds a [`UniformLayout`] from fields in declaration order, computing
/// each offset from WGSL's own alignment rule and padding the total to a
/// multiple of 16.
pub struct UniformLayoutBuilder {
    fields: Vec<UniformField>,
    offset: u32,
}

impl UniformLayoutBuilder {
    pub fn new() -> UniformLayoutBuilder {
        UniformLayoutBuilder {
            fields: Vec::new(),
            offset: 0,
        }
    }

    /// Adds one of this crate's own fixed fields (`t`, `dt`, `capacity`, …).
    /// The WGSL spelling is `name` verbatim — safe because this crate always
    /// chooses these names itself and `.kir` text never gets to pick them.
    pub fn field(&mut self, name: impl Into<String>, wgsl_ty: &'static str) -> &mut Self {
        let name = name.into();
        let wgsl_name = name.clone();
        self.push(name, wgsl_name, wgsl_ty)
    }

    /// Adds a field for a declared `param`. The WGSL spelling is always
    /// mangled — see [`mangle_param`].
    pub fn param_field(&mut self, name: impl Into<String>, wgsl_ty: &'static str) -> &mut Self {
        let name = name.into();
        let wgsl_name = mangle_param(&name);
        self.push(name, wgsl_name, wgsl_ty)
    }

    /// Adds a field for a **field's** declared `param` — see
    /// [`mangle_field_param`]. The semantic `name` is prefixed too, because the
    /// engine looks a uniform field up by that name and a Set may hold a field
    /// and a renderer that declare the same one.
    pub fn field_param_field(
        &mut self,
        slot: &str,
        name: &str,
        wgsl_ty: &'static str,
    ) -> &mut Self {
        self.push(
            field_param_key(slot, name),
            mangle_field_param(slot, name),
            wgsl_ty,
        )
    }

    /// Adds the `u32` a declared Source slot is read out of — see
    /// [`mangle_source_slot`]. Both names are prefixed, for the reason
    /// [`UniformLayoutBuilder::field_param_field`]'s are.
    pub fn source_slot_field(&mut self, slot: &str) -> &mut Self {
        self.push(source_slot_key(slot), mangle_source_slot(slot), "u32")
    }

    fn push(&mut self, name: String, wgsl_name: String, wgsl_ty: &'static str) -> &mut Self {
        let (align, size) = align_size(wgsl_ty);
        let offset = align_up(self.offset, align);
        self.fields.push(UniformField {
            name,
            wgsl_name,
            wgsl_ty,
            offset,
            size,
        });
        self.offset = offset + size;
        self
    }

    /// Finishes the layout, padding to a 16-byte multiple. Returns the layout and trailing pad count.
    pub fn finish(self) -> (UniformLayout, u32) {
        let total_size = align_up(self.offset, 16);
        let pad_bytes = total_size - self.offset;
        debug_assert_eq!(pad_bytes % 4, 0);
        (
            UniformLayout {
                fields: self.fields,
                total_size,
            },
            pad_bytes / 4,
        )
    }
}

impl Default for UniformLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Emits `struct Uniforms { ... };` in WGSL, adding explicit scalar padding fields as needed.
pub fn write_uniform_struct(out: &mut String, layout: &UniformLayout, pad_f32: u32) {
    out.push_str("struct Uniforms {\n");
    for f in &layout.fields {
        out.push_str(&format!("    {}: {},\n", f.wgsl_name, f.wgsl_ty));
    }
    match pad_f32 {
        0 => {}
        1 => out.push_str("    _pad: f32,\n"),
        n => {
            for i in 0..n {
                out.push_str(&format!("    _pad{i}: f32,\n"));
            }
        }
    }
    out.push_str("};\n");
}

#[cfg(test)]
mod tests {
    use karakuri_ir::layout::{generate_element_layout, Synthetic};
    use karakuri_ir::Attr;

    use super::*;

    #[test]
    fn write_element_struct_matches_the_layout_field_order() {
        let layout = generate_element_layout(&[Attr::Position, Attr::Tint], Synthetic::NONE, &[]);
        let mut out = String::new();
        write_element_struct(&mut out, &layout);
        assert!(out.contains("seed: u32,"), "{out}");
        assert!(out.contains("birth_frac: f32,"), "{out}");
        assert!(out.contains("position: vec3<f32>,"), "{out}");
        assert!(out.contains("tint: vec3<f32>,"), "{out}");
        // Declaration order must match `slots`, not merely contain them.
        let seed_at = out.find("seed:").unwrap();
        let birth_at = out.find("birth_frac:").unwrap();
        let pos_at = out.find("position:").unwrap();
        let tint_at = out.find("tint:").unwrap();
        assert!(
            seed_at < birth_at && birth_at < pos_at && pos_at < tint_at,
            "{out}"
        );
    }

    #[test]
    fn vec3_field_after_scalar_gets_padded_to_16_align() {
        let mut b = UniformLayoutBuilder::new();
        b.field("t", "f32");
        b.field("glow", "vec3<f32>");
        let (layout, _pad) = b.finish();
        assert_eq!(layout.fields[0].offset, 0);
        // vec3 aligns to 16, so it cannot start at byte 4.
        assert_eq!(layout.fields[1].offset, 16);
    }

    #[test]
    fn padding_of_two_or_three_f32_never_becomes_an_array() {
        // WGSL requires uniform array elements to have a 16-byte aligned stride.
        // `array<f32, N>` has a 4-byte stride, so multi-slot padding uses scalar fields.
        for param_count in 0..6 {
            let mut b = UniformLayoutBuilder::new();
            b.field("t", "f32");
            for i in 0..param_count {
                b.field(format!("p{i}"), "f32");
            }
            let (layout, pad_f32) = b.finish();
            let mut out = String::new();
            write_uniform_struct(&mut out, &layout, pad_f32);
            assert!(
                !out.contains("array<f32"),
                "param_count={param_count}:\n{out}"
            );
        }
    }

    /// Verifies that uniform alignment and size match storage element alignment rules.
    #[test]
    fn align_size_defers_to_the_element_table_for_every_type_it_knows() {
        for elem in StorageElemTy::ALL {
            assert_eq!(
                align_size(elem.wgsl_name()),
                (elem.align(), elem.size()),
                "{}",
                elem.wgsl_name()
            );
        }
    }

    /// The other half of [`align_size`]'s job: the types no element holds, for
    /// which there is no variant to defer to and this crate is the only owner.
    #[test]
    fn align_size_still_places_the_types_no_element_holds() {
        assert!(StorageElemTy::from_wgsl_name("i32").is_none());
        assert_eq!(align_size("i32"), (4, 4));
        assert_eq!(align_size("vec4<f32>"), (16, 16));
        assert_eq!(align_size("mat4x4<f32>"), (16, 64));
    }

    #[test]
    fn total_size_is_sixteen_byte_aligned() {
        for extra in 0..8 {
            let mut b = UniformLayoutBuilder::new();
            b.field("t", "f32");
            for i in 0..extra {
                b.field(format!("p{i}"), "f32");
            }
            let (layout, _pad) = b.finish();
            assert_eq!(layout.total_size % 16, 0, "extra={extra}");
        }
    }
}
