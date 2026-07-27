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
//! # Per-element state: one buffer per direction, not one per attribute
//!
//! Every emitted attribute used to get its own pair of storage buffers, so a
//! procedure emitting three attributes bound 12 storage buffers in its
//! compute stage (3 synthetic + 3 declared, times prev and next) — more than
//! the WebGPU default limit of 8, and twice the downlevel default of 4. All
//! per-element state that lives in the *same* buffer as everything else of
//! its own direction packs into one generated `Element` struct instead: one
//! `vec4` slot per entry (`seed`, `birth_frac`, then `emit` in declaration
//! order), 16 bytes each, so `stride = (2 + emit.len()) * 16` sizes the
//! buffer with no per-attribute case analysis — see [`ElementSlot`] for why
//! every slot is still a full `vec4` rather than its attribute's natural
//! width.
//!
//! `alive` is the one exception: it leaves the struct entirely and becomes
//! its own tight `array<u32>` buffer, 4 bytes per element, because the
//! compaction scan (`crates/karakuri-engine/src/shaders/scan.wgsl`) reads
//! alive flags as a dense array with no stride arithmetic. Packing it into
//! `Element` would mean the scan needs to know the struct's per-procedure
//! size just to skip to the next flag; a dense array needs nothing but the
//! element count.
//!
//! # The counts buffer: engine state the entry points cannot work without
//!
//! Neither entry point can know how far the live range extends, and since
//! compaction moved that number onto the GPU nothing host-side knows it
//! either. One storage buffer — [`counts`] — carries it, alongside the
//! indirect arguments derived from it, and is bound read-only to both L1
//! entry points at group [`group::UNIFORMS`], binding [`binding::COUNTS`].
//! It is engine state neither entry point can compute, exactly like `dt` and
//! `capacity` in the uniform buffer beside it, which is why it shares that
//! group rather than getting one of its own — but unlike them it is written
//! on the GPU and changes once per *step*, not once per frame. The per-step
//! quantities the host does still own are in [`step_args`].
//!
//! # L1 (compute)
//!
//! - Group [`group::UNIFORMS`], binding [`binding::UNIFORM`]: the uniform
//!   buffer. Binding [`binding::COUNTS`]: the [`counts`] buffer, read-only
//!   storage. Binding [`binding::DEST`]: `array<u32>`, the scan's
//!   destination indices, read-only storage — **present only for a
//!   compacted procedure**, since a static one's `element` writes in place
//!   and must not pay for a buffer read it cannot need.
//! - Group [`group::PREV`]: binding [`binding::ELEMENT`] is `array<Element>`
//!   read-only, the previous frame's values; binding [`binding::ALIVE`] is
//!   `array<u32>` read-only, the previous frame's alive flags.
//! - Group [`group::NEXT`]: the same two bindings, holding the next frame's
//!   values, both read-write.
//! - Group [`group::STEP`], binding [`binding::UNIFORM`]: the
//!   [`step_args`] uniform, bound at a per-substep offset. `spawn` only.
//!
//! Double buffering is not a binding-number swap: the two groups keep fixed
//! roles ("prev" / "next") in the shader, and the engine swaps which physical
//! buffer backs each group's bind group between frames. This is the standard
//! ping-pong idiom and it means the generated WGSL never has to change to
//! participate in it.
//!
//! Both `spawn` and `element` use [`WORKGROUP_SIZE`]. `element` dispatches
//! indirectly over `counts.range` and writes each survivor at `dest[i]` (or
//! at `i`, for a static procedure); `spawn` dispatches over the frame's
//! new-element count and writes at `counts.survivors + invocation`. The
//! prefix-sum scan that fills `dest` is engine infrastructure this crate
//! does not generate — see the module doc on `l1.rs` for why.
//!
//! # L4 (render)
//!
//! - Group [`group::UNIFORMS`], binding [`binding::UNIFORM`]: the
//!   `L4Uniforms` uniform buffer (see [`crate::l4`]).
//! - Group [`group::ATTRS`], binding [`binding::ELEMENT`]: `array<Element>`
//!   read-only, indexed by `@builtin(instance_index)`. This is the L1 side's
//!   [`group::PREV`] element buffer from the frame just computed — L4 never
//!   sees "next", only the current, already-swapped state. Binding
//!   [`binding::ALIVE`]: the matching alive flags, also read-only. L4 draws
//!   `counts.range` instances, which is not the alive count: elements killed
//!   during the step that just ran are scattered anywhere through
//!   `[0, counts.survivors)` — compaction placed them by their pre-kill
//!   position, and the kill only cleared a flag — and still occupy those
//!   slots until the next step's scan reclaims them. So the vertex stage has
//!   to read the flag per instance and skip them, not trim a tail.
//!
//! L4's `Element` struct must be **byte-identical** to the L1 procedure it is
//! paired with, because it reads the same physical buffer L1 wrote — see
//! [`generate_element_layout`] and [`crate::l4::generate_l4`]'s signature,
//! which takes the layout rather than deriving its own.
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

    /// A uniform buffer: `Uniforms` in [`group::UNIFORMS`], `StepArgs` in
    /// [`group::STEP`](super::group::STEP).
    pub const UNIFORM: u32 = 0;
    /// [`group::UNIFORMS`]: the [`super::counts`] buffer, read-only storage.
    pub const COUNTS: u32 = 1;
    /// [`group::UNIFORMS`]: the scan's `array<u32>` destination indices,
    /// read-only storage. Compacted procedures only.
    pub const DEST: u32 = 2;
}

/// The engine's per-frame count state, and the indirect arguments derived
/// from it, in one buffer.
///
/// One buffer rather than three because every consumer of one of these
/// numbers is a consumer of another: the scan writes `survivors`, `advance`
/// turns that into `range` and rewrites the two argument blocks from it, and
/// `element` reads `range` in the same step that the scan wrote `survivors`.
/// Splitting them would mean three buffers whose contents can only ever be
/// read together.
///
/// The two argument blocks are at fixed offsets because the GPU reads them
/// as arguments, not as struct fields: `dispatch_workgroups_indirect` wants
/// three `u32`s at [`counts::ELEM_XYZ`] and `draw_indirect` wants four at
/// [`counts::DRAW`], both in wgpu's own argument order. `range` sits in the
/// fourth word of the dispatch block precisely because that word is not part
/// of the dispatch arguments and is therefore free.
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

/// Per-substep spawn parameters: the one piece of engine state that differs
/// between the substeps of a single frame.
///
/// Per-substep state, as against the per-frame state in the uniform buffer.
///
/// The dividing line is **input versus simulation state**. A `param`, a signal
/// binding, and the camera are external control, genuinely sampled once per
/// frame, and they stay in the uniform. `t` and the spawn count are the
/// simulation's own clock, and they advance once per step or substepping does
/// not do the one thing it exists for: a frame of two steps has to put the
/// simulation exactly where two frames of one step would. Holding `t` constant
/// across substeps runs both passes at the same instant, which any procedure
/// reading `t` can see; batching a frame's spawns into its first substep puts
/// two frames of elements in before the second `element` pass.
///
/// A `queue.write_buffer` cannot be interleaved between commands already in an
/// encoder, so the engine writes every substep's entry up front, at
/// [`step_args::STRIDE`] apart, and binds the right one per substep.
pub mod step_args {
    /// Byte size of one entry.
    pub const SIZE: u64 = 16;
    /// Distance between consecutive substeps' entries. 256 is the WebGPU
    /// default `min_uniform_buffer_offset_alignment` and a multiple of every
    /// smaller value an adapter may report, so a binding at `k * STRIDE` is
    /// always legally aligned.
    pub const STRIDE: u64 = 256;

    /// The WGSL declaration, shared with the engine's `advance` pass for the
    /// same reason [`super::counts::WGSL`] is.
    pub const WGSL: &str = "\
struct StepArgs {
    spawn_count: u32,
    seed_base: u32,
    capacity: u32,
    t: f32,
};
";
}

/// The WGSL element type an `Element` struct field is declared with. Every
/// field uses one of these two, never the attribute's "natural" narrower
/// type — see [`ElementSlot`] for why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageElemTy {
    /// `vec4<f32>`, 16 bytes.
    Vec4F32,
    /// `vec4<u32>`, 16 bytes.
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

/// One `vec4` field of the generated `Element` struct.
///
/// Every field — including the two synthetic ones no procedure ever names —
/// is `vec4<f32>` or `vec4<u32>` regardless of the attribute's own width.
/// `position` (`vec3`) only needs 12 of its 16 bytes; `age` (`float`) only
/// needs 4. The waste buys one property worth more than the bytes: every
/// field has the same 16-byte size, so `(2 + emit.len()) * 16` is the
/// stride of *every* `Element` buffer the engine allocates, with no
/// per-attribute case analysis and no possibility of the naturally-narrower
/// types (`vec2`, `float`, `u32`) producing a size the spec's "16-byte
/// aligned" would not obviously cover. `vec3` already gets this for free
/// from WGSL's own array-stride rule; this makes it uniform instead of an
/// exception, and it is a deliberate choice to keep, not an oversight this
/// task happens to touch — natural widths with computed offsets are a
/// separate decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementSlot {
    /// The struct field name, and the WGSL identifier used everywhere the
    /// slot is read or written (`prev[i].<name>`, `next[i].<name>`,
    /// `elements[elem].<name>`). Every possible value — `seed`,
    /// `birth_frac`, and every [`Attr`] name — is already a valid WGSL
    /// identifier and never a reserved word, so unlike a `param` this needs
    /// no mangling.
    pub name: &'static str,
    /// `None` for the two attributes no procedure can name: `seed` and the
    /// birth fraction. They are allocated unconditionally because identity
    /// and spawn timing need them whether or not the procedure's `emit` list
    /// does.
    pub attr: Option<Attr>,
    pub elem_ty: StorageElemTy,
    /// Byte offset of this field within one `Element` entry. Always
    /// `index * 16`, published rather than left for a caller to recompute —
    /// same reasoning as [`UniformField::offset`].
    pub offset: u32,
}

/// The ordered slots of a generated `Element` struct, plus the struct's
/// byte stride. This is the contract `karakuri-engine` drives: it sizes the
/// element buffer as `capacity * stride`, and [`ElementLayout::offset_of`]
/// is how it finds where a given slot's bytes land when it needs to write
/// one directly (initializing `seed` and `birth_frac` for a spawn-less
/// procedure — see `karakuri-engine::set::Set::initialize`).
#[derive(Debug, Clone, PartialEq)]
pub struct ElementLayout {
    /// `seed`, then `birth_frac`, then `emit` in declaration order — see
    /// [`generate_element_layout`].
    pub slots: Vec<ElementSlot>,
    /// `slots.len() as u32 * 16`.
    pub stride: u32,
}

impl ElementLayout {
    /// The byte offset of the slot named `name` within one `Element` entry.
    /// Panics if no such slot exists — every caller asks for `seed` or
    /// `birth_frac`, which [`generate_element_layout`] always allocates.
    pub fn offset_of(&self, name: &str) -> u32 {
        self.slots
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("ElementLayout has no slot named `{name}`"))
            .offset
    }
}

fn attr_elem_ty(attr: Attr) -> StorageElemTy {
    // Every declarable attribute (`Attr::ty`) is float, vec2, or vec3 — see
    // `karakuri-ir/src/ast.rs`. None of them is ever uint, so `Vec4F32`
    // covers all of them; only the synthetic `seed` slot needs `Vec4U32`.
    match attr.ty() {
        Ty::Float | Ty::Vec2 | Ty::Vec3 => StorageElemTy::Vec4F32,
        other => unreachable!("attribute {} has non-float-family type {:?}", attr.name(), other),
    }
}

/// Builds the `Element` struct layout for an L1 procedure's `emit` list:
/// `seed` first, then `birth_frac`, then `emit` in declaration order. Fixed
/// order because it is the shape of a struct in WGSL text, not a set of
/// independently addressable bindings — unlike the old per-attribute
/// binding numbers, there is no freedom to reorder without changing what
/// every `prev[i].<field>` access compiles to.
///
/// L4 must be generated against the exact [`ElementLayout`] this returns for
/// the L1 procedure it is paired with, since it reads the same physical
/// buffer L1 wrote — see [`crate::l4::generate_l4`].
pub fn generate_element_layout(emit: &[Attr]) -> ElementLayout {
    let seed = ("seed", None, StorageElemTy::Vec4U32);
    let birth_frac = ("birth_frac", None, StorageElemTy::Vec4F32);
    let declared = emit.iter().map(|&attr| (attr.name(), Some(attr), attr_elem_ty(attr)));
    let slots: Vec<ElementSlot> = std::iter::once(seed)
        .chain(std::iter::once(birth_frac))
        .chain(declared)
        .enumerate()
        .map(|(i, (name, attr, elem_ty))| ElementSlot { name, attr, elem_ty, offset: i as u32 * 16 })
        .collect();
    let stride = slots.len() as u32 * 16;
    ElementLayout { slots, stride }
}

/// Writes `struct Element { ... };` for `layout`. Shared by [`crate::l1`]
/// and [`crate::l4`] so the two crate-internal call sites can never drift —
/// the whole point of L4 taking an [`ElementLayout`] instead of deriving one
/// from `consumes` is that this text has to be byte-identical between the
/// two, since they address the same physical buffer.
pub fn write_element_struct(out: &mut String, layout: &ElementLayout) {
    out.push_str("struct Element {\n");
    for s in &layout.slots {
        out.push_str(&format!("    {}: {},\n", s.name, s.elem_ty.wgsl_name()));
    }
    out.push_str("};\n");
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
/// the other half of the "byte offsets" contract alongside [`ElementSlot`] —
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
    fn element_layout_puts_seed_and_birth_frac_first_at_fixed_offsets() {
        let layout = generate_element_layout(&[Attr::Position, Attr::Age]);
        assert_eq!(layout.slots[0].name, "seed");
        assert_eq!(layout.slots[0].offset, 0);
        assert_eq!(layout.slots[1].name, "birth_frac");
        assert_eq!(layout.slots[1].offset, 16);
        assert_eq!(layout.slots[2].attr, Some(Attr::Position));
        assert_eq!(layout.slots[2].offset, 32);
        assert_eq!(layout.slots[3].attr, Some(Attr::Age));
        assert_eq!(layout.slots[3].offset, 48);
    }

    #[test]
    fn element_layout_stride_is_two_plus_emit_len_times_sixteen() {
        assert_eq!(generate_element_layout(&[]).stride, 32);
        assert_eq!(generate_element_layout(&[Attr::Position]).stride, 48);
        assert_eq!(generate_element_layout(&[Attr::Position, Attr::Velocity, Attr::Age]).stride, 80);
    }

    #[test]
    fn offset_of_finds_the_synthetic_slots() {
        let layout = generate_element_layout(&[Attr::Position]);
        assert_eq!(layout.offset_of("seed"), 0);
        assert_eq!(layout.offset_of("birth_frac"), 16);
        assert_eq!(layout.offset_of("position"), 32);
    }

    #[test]
    fn every_slot_is_sixteen_bytes_regardless_of_attribute_width() {
        // The whole point of wrapping every field in vec4: no per-attribute
        // case analysis is needed to compute a slot's size.
        for attr in Attr::ALL {
            let ty = attr_elem_ty(attr);
            assert!(matches!(ty, StorageElemTy::Vec4F32 | StorageElemTy::Vec4U32));
        }
    }

    #[test]
    fn write_element_struct_matches_the_layout_field_order() {
        let layout = generate_element_layout(&[Attr::Position, Attr::Tint]);
        let mut out = String::new();
        write_element_struct(&mut out, &layout);
        assert!(out.contains("seed: vec4<u32>,"), "{out}");
        assert!(out.contains("birth_frac: vec4<f32>,"), "{out}");
        assert!(out.contains("position: vec4<f32>,"), "{out}");
        assert!(out.contains("tint: vec4<f32>,"), "{out}");
        // Declaration order must match `slots`, not merely contain them.
        let seed_at = out.find("seed:").unwrap();
        let birth_at = out.find("birth_frac:").unwrap();
        let pos_at = out.find("position:").unwrap();
        let tint_at = out.find("tint:").unwrap();
        assert!(seed_at < birth_at && birth_at < pos_at && pos_at < tint_at, "{out}");
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
