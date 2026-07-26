//! The binding layout contract.
//!
//! This module is documentation with a compiler behind it. `karakuri-engine`
//! drives whatever WGSL `karakuri-codegen` emits by creating buffers, bind
//! groups, and pipelines that match these group/binding numbers, this
//! uniform field order, and this workgroup size exactly — none of it is
//! meant to be rediscovered by reading the generated WGSL text. Treat it as
//! an API, not an implementation detail: a change here is a breaking change
//! for the engine.
//!
//! # L1 (compute)
//!
//! - Group [`group::UNIFORMS`], binding 0: the [`L1Uniforms`] uniform buffer.
//! - Group [`group::PREV`]: one storage buffer per allocated attribute slot
//!   (see [`AttrSlot`]), holding the previous frame's values. Bound read-only.
//! - Group [`group::NEXT`]: the same slots, same binding numbers, holding the
//!   next frame's values. Bound read-write.
//!
//! Double buffering is not a binding-number swap: the two groups keep fixed
//! roles ("prev" / "next") in the shader, and the engine swaps which physical
//! buffer backs each group's bind group between frames. This is the standard
//! ping-pong idiom and it means the generated WGSL never has to change to
//! participate in it.
//!
//! Both `spawn` and `element` use [`WORKGROUP_SIZE`]. `spawn` dispatches over
//! the frame's new-element count and writes at `live_count + invocation`;
//! `element` dispatches over the previous live count and writes at
//! `invocation` (see `l1.rs` — order-preserving compaction via a prefix-sum
//! scan is engine infrastructure this crate does not generate; see the
//! module doc there for why).
//!
//! # L4 (render)
//!
//! - Group [`group::UNIFORMS`], binding 0: the `L4Uniforms` uniform buffer
//!   (see [`crate::l4`]).
//! - Group [`group::ATTRS`]: one storage buffer per **consumed** attribute
//!   plus `seed`, read-only, indexed by `@builtin(instance_index)`. This is
//!   the L1 side's [`group::PREV`] buffer set from the frame just computed —
//!   L4 never sees "next", only the current, already-swapped state.
//!
//! Six vertices per instance (`@builtin(vertex_index)` 0..6), no vertex
//! buffers: see the L4 module doc for the quad-expansion this exists for.

use karakuri_ir::{Attr, Ty};

/// Every compute entry point (`spawn`, `element`) uses this workgroup size.
pub const WORKGROUP_SIZE: u32 = 64;

/// Vertices emitted per element by the L4 vertex stage. WebGPU has no point
/// size, so `topology points` expands to two triangles covering a quad
/// rather than to `PrimitiveTopology::PointList`.
pub const VERTICES_PER_ELEMENT: u32 = 6;

/// Bind group indices. Shared constants so L1 and L4 code (and the engine)
/// spell the same number the same way.
pub mod group {
    /// The uniform buffer, both stages.
    pub const UNIFORMS: u32 = 0;
    /// L1 only: previous-frame attribute buffers, read-only.
    pub const PREV: u32 = 1;
    /// L1 only: next-frame attribute buffers, read-write.
    pub const NEXT: u32 = 2;
    /// L4 only: the current attribute buffers (L1's post-swap `PREV` set),
    /// read-only. Reuses binding number 1 because a shader module is either
    /// L1 or L4, never both, so the numbers never collide in one pipeline.
    pub const ATTRS: u32 = 1;
}

/// The WGSL element type a storage buffer is declared with. Every attribute
/// buffer uses one of these two, never the attribute's "natural" narrower
/// type — see the module doc on why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageElemTy {
    /// `array<vec4<f32>>`, stride 16.
    Vec4F32,
    /// `array<vec4<u32>>`, stride 16.
    Vec4U32,
}

impl StorageElemTy {
    pub fn wgsl_name(self) -> &'static str {
        match self {
            StorageElemTy::Vec4F32 => "vec4<f32>",
            StorageElemTy::Vec4U32 => "vec4<u32>",
        }
    }
}

/// One per-element storage buffer, allocated in a prev/next pair on L1 and a
/// single read-only copy on L4.
///
/// Every attribute buffer — including the three synthetic ones no procedure
/// ever names — is declared `array<vec4<f32>>` or `array<vec4<u32>>`
/// regardless of the attribute's own width. `position` (`vec3`) only needs
/// 12 of its 16 bytes; `age` (`float`) only needs 4. The waste buys one
/// property worth more than the bytes: every attribute buffer has the same
/// 16-byte per-element stride, so `capacity * 16` is the size of *every*
/// buffer the engine allocates, with no per-attribute case analysis and no
/// possibility of the naturally-narrower types (`vec2`, `float`, `u32`)
/// producing a stride the spec's "16-byte aligned" would not obviously cover.
/// `vec3` already gets this for free from WGSL's own array-stride rule; this
/// makes it uniform instead of an exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttrSlot {
    /// The WGSL identifier fragment used to name this attribute's buffers
    /// (`prev_<name>` / `next_<name>` on L1, `attr_<name>` on L4).
    pub name: &'static str,
    /// `None` for the three attributes no procedure can name: `seed`, the
    /// alive flag, and the birth fraction. They are allocated unconditionally
    /// because spawn timing and compaction need them whether or not the
    /// procedure's `emit` list does.
    pub attr: Option<Attr>,
    /// Binding number within [`group::PREV`] / [`group::NEXT`] (L1) or
    /// [`group::ATTRS`] (L4, consumed attributes only — see
    /// [`l4_attr_slots`]).
    pub binding: u32,
    pub elem_ty: StorageElemTy,
}

/// The always-allocated attribute names, in the fixed order they occupy the
/// first bindings of every L1 attribute group. `seed` and the alive flag are
/// `u32`-backed; the birth fraction is `f32`-backed.
const SYNTHETIC: [(&str, StorageElemTy); 3] = [
    ("seed", StorageElemTy::Vec4U32),
    ("alive", StorageElemTy::Vec4U32),
    ("birth_frac", StorageElemTy::Vec4F32),
];

fn attr_elem_ty(attr: Attr) -> StorageElemTy {
    // Every declarable attribute (`Attr::ty`) is float, vec2, or vec3 — see
    // `karakuri-ir/src/ast.rs`. None of them is ever uint, so `Vec4F32`
    // covers all of them; only the synthetic `seed`/`alive` slots need
    // `Vec4U32`.
    match attr.ty() {
        Ty::Float | Ty::Vec2 | Ty::Vec3 => StorageElemTy::Vec4F32,
        other => unreachable!("attribute {} has non-float-family type {:?}", attr.name(), other),
    }
}

/// The full ordered list of attribute slots for an L1 procedure: the three
/// synthetic ones first, then `emit` in declaration order. Binding numbers
/// are assigned by position in this list and are identical between
/// [`group::PREV`] and [`group::NEXT`] — only which physical buffer backs
/// each group's bind group differs, and that is the engine's swap, not a
/// binding-number change.
pub fn l1_attr_slots(emit: &[Attr]) -> Vec<AttrSlot> {
    let synthetic = SYNTHETIC.into_iter().map(|(name, elem_ty)| (name, None, elem_ty));
    let declared = emit.iter().map(|&attr| (attr.name(), Some(attr), attr_elem_ty(attr)));
    synthetic
        .chain(declared)
        .enumerate()
        .map(|(binding, (name, attr, elem_ty))| AttrSlot { name, attr, binding: binding as u32, elem_ty })
        .collect()
}

/// The attribute slots an L4 procedure reads: `seed` always (identity is
/// always potentially in scope), then `consumes` in declaration order. There
/// is no "next" side — L4 is stateless and reads whatever L1 last produced.
pub fn l4_attr_slots(consumes: &[Attr]) -> Vec<AttrSlot> {
    let seed = ("seed", None, StorageElemTy::Vec4U32);
    let declared = consumes.iter().map(|&attr| (attr.name(), Some(attr), attr_elem_ty(attr)));
    std::iter::once(seed)
        .chain(declared)
        .enumerate()
        .map(|(binding, (name, attr, elem_ty))| AttrSlot { name, attr, binding: binding as u32, elem_ty })
        .collect()
}

/// One field of a generated uniform struct, with the byte offset it was
/// placed at. Offsets follow WGSL's own uniform-address-space layout rules
/// (`vec3` aligns to 16 despite being 12 bytes, etc.) — this struct records
/// what the generator computed, it does not invent a different layout than
/// the WGSL text it emits.
#[derive(Debug, Clone, PartialEq)]
pub struct UniformField {
    /// The semantic name: a declared `param`'s own name verbatim, or one of
    /// this crate's fixed engine fields (`t`, `dt`, `capacity`, …). This is
    /// what `karakuri-engine`'s uniform packer looks a field up by — a Set
    /// record names a param by its declared `.kir` name
    /// (`{"t":"param","key":"radius",...}`), never by `wgsl_name`, so the
    /// packer's lookups have to key on this field, not on the WGSL spelling.
    pub name: String,
    /// The identifier actually written in the WGSL struct declaration and
    /// at every `u.<field>` read site. Equal to `name` for this crate's own
    /// fixed fields, since it always chooses those itself and they are
    /// never a WGSL reserved word by construction. For a `param`, always
    /// [`mangle_param`]`(name)` — see there for why every param is mangled,
    /// not only the ones that happen to collide with a reserved word today.
    pub wgsl_name: String,
    /// WGSL type spelling, e.g. `"f32"`, `"vec3<f32>"`, `"mat4x4<f32>"`.
    pub wgsl_ty: &'static str,
    pub offset: u32,
    pub size: u32,
}

/// Mangles a `param`'s declared name into the identifier this crate writes
/// for its uniform struct field, at both the declaration site and every
/// `u.<field>` read.
///
/// Unconditionally — for every param, not only the ones that happen to
/// collide with a WGSL reserved word today. WGSL reserves a long list of
/// identifiers `.kir` does not (`array`, `struct`, `loop`, `switch`,
/// `return`, `discard`, `const`, `override`, `enable`, `bitcast`, `fn`,
/// `atomic`, `ptr`, `sampler`, and more), and a generator that only mangled
/// names it recognised from a blocklist would need to track the WGSL
/// specification forever to stay correct as it grows. That is exactly the
/// reasoning that made blanket mangling the right call for `let`/`var`
/// locals rather than trying to enumerate which spellings a procedure might
/// plausibly reach for — `param array : float ...` is not contrived, a
/// generator writing a procedure about a particle array reaches for exactly
/// that word.
///
/// A different prefix from [`crate::lower::mangle_local`]'s (`param_` here,
/// `usr_` there) purely so a human reading generated WGSL can tell at a
/// glance which kind of `.kir` declaration a name came from. It does not
/// matter for correctness: a param is always read through `u.` field
/// access and a local is always a bare identifier, so the two namespaces
/// cannot collide with each other even sharing one prefix.
pub fn mangle_param(name: &str) -> String {
    format!("param_{name}")
}

/// The complete field order and size of a generated uniform struct. This is
/// the other half of the "byte offsets" contract alongside [`AttrSlot`] —
/// `karakuri-engine` packs the CPU-side struct that gets uploaded to this
/// binding by walking `fields` in order, not by guessing at WGSL's layout
/// rules independently.
#[derive(Debug, Clone, PartialEq)]
pub struct UniformLayout {
    pub fields: Vec<UniformField>,
    /// Always a multiple of 16. A uniform buffer binding whose size is not
    /// is a validation error reported nowhere near this function, so the
    /// layout builder pads to it unconditionally rather than leaving it to
    /// chance — see the `total_size_is_sixteen_byte_aligned` test.
    pub total_size: u32,
}

fn align_up(offset: u32, align: u32) -> u32 {
    offset.div_ceil(align) * align
}

/// WGSL uniform-address-space `(align, size)` for the scalar/vector/matrix
/// types the uniform structs this crate emits ever use. Not a general WGSL
/// layout function — only the types [`crate::l1`] and [`crate::l4`] put in a
/// uniform struct appear here.
pub fn align_size(wgsl_ty: &str) -> (u32, u32) {
    match wgsl_ty {
        "f32" | "u32" | "i32" => (4, 4),
        "vec2<f32>" => (8, 8),
        "vec3<f32>" => (16, 12),
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
        UniformLayoutBuilder { fields: Vec::new(), offset: 0 }
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

    fn push(&mut self, name: String, wgsl_name: String, wgsl_ty: &'static str) -> &mut Self {
        let (align, size) = align_size(wgsl_ty);
        let offset = align_up(self.offset, align);
        self.fields.push(UniformField { name, wgsl_name, wgsl_ty, offset, size });
        self.offset = offset + size;
        self
    }

    /// Finishes the layout, padding to a 16-byte multiple. Returns the
    /// layout plus the number of trailing `f32` pad slots to declare in
    /// WGSL — see [`write_uniform_struct`] for how those are spelled
    /// (omitted from `fields` either way: it is wire padding, not a value
    /// the engine ever sets).
    pub fn finish(self) -> (UniformLayout, u32) {
        let total_size = align_up(self.offset, 16);
        let pad_bytes = total_size - self.offset;
        debug_assert_eq!(pad_bytes % 4, 0);
        (UniformLayout { fields: self.fields, total_size }, pad_bytes / 4)
    }
}

impl Default for UniformLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Writes `struct Uniforms { ... };` for `layout`, plus `pad_f32` trailing
/// pad fields. Shared by [`crate::l1`] and [`crate::l4`] — both build a
/// [`UniformLayout`] and need identical WGSL for it.
///
/// Padding is emitted as individually named scalar fields (`_pad0`,
/// `_pad1`, …; just `_pad` when there is exactly one), never as
/// `array<f32, N>`. WGSL requires array elements inside a uniform-address-
/// space struct to have a stride that is itself a multiple of 16 — true of
/// every *attribute* array this crate emits (`array<vec4<f32>>`, by
/// design), but `array<f32, N>` has a 4-byte stride and fails validation
/// the moment `pad_f32` is 2 or 3, which single-field padding never
/// triggers because a lone scalar field is not an array at all.
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
    use super::*;

    #[test]
    fn l1_synthetic_slots_come_first_at_fixed_bindings() {
        let slots = l1_attr_slots(&[Attr::Position, Attr::Age]);
        assert_eq!(slots[0].name, "seed");
        assert_eq!(slots[0].binding, 0);
        assert_eq!(slots[1].name, "alive");
        assert_eq!(slots[1].binding, 1);
        assert_eq!(slots[2].name, "birth_frac");
        assert_eq!(slots[2].binding, 2);
        assert_eq!(slots[3].attr, Some(Attr::Position));
        assert_eq!(slots[3].binding, 3);
        assert_eq!(slots[4].attr, Some(Attr::Age));
        assert_eq!(slots[4].binding, 4);
    }

    #[test]
    fn l4_slots_have_seed_then_consumed_attrs() {
        let slots = l4_attr_slots(&[Attr::Position, Attr::Velocity]);
        assert_eq!(slots[0].name, "seed");
        assert_eq!(slots[1].attr, Some(Attr::Position));
        assert_eq!(slots[2].attr, Some(Attr::Velocity));
    }

    #[test]
    fn every_attribute_buffer_is_sixteen_byte_stride() {
        // The whole point of wrapping every attribute in vec4: capacity * 16
        // is the size of every buffer, no per-attribute case analysis.
        for attr in Attr::ALL {
            let ty = attr_elem_ty(attr);
            assert!(matches!(ty, StorageElemTy::Vec4F32 | StorageElemTy::Vec4U32));
        }
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
        // WGSL requires array elements inside a uniform-address-space struct
        // to have a stride that is itself a multiple of 16; `array<f32, N>`
        // has a 4-byte stride and fails validation for N >= 2, so multi-slot
        // padding must be individually named scalar fields instead. A single
        // pad field (N == 1) was already covered before this was caught by
        // an end-to-end naga run, which is exactly the gap this locks shut.
        for param_count in 0..6 {
            let mut b = UniformLayoutBuilder::new();
            b.field("t", "f32");
            for i in 0..param_count {
                b.field(format!("p{i}"), "f32");
            }
            let (layout, pad_f32) = b.finish();
            let mut out = String::new();
            write_uniform_struct(&mut out, &layout, pad_f32);
            assert!(!out.contains("array<f32"), "param_count={param_count}:\n{out}");
        }
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
