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
//! # Per-element state: placed elsewhere, bound here
//!
//! Where an element's bytes go is not this module's rule any more. All
//! per-element state that lives in the *same* buffer as everything else of its
//! own direction packs into one generated `Element` struct — one slot per
//! entry, each at its own width and at the offset WGSL's layout rules give it —
//! and `alive` leaves the struct entirely for its own tight `array<u32>` at
//! [`karakuri_ir::layout::ALIVE_BYTES`] per element. Both rules live in
//! [`karakuri_ir::layout`], whose module doc argues them; they moved there
//! because stage 4's cost estimate has to charge what the engine allocates and
//! cannot call a crate that depends on it, so it kept a second copy of the
//! arithmetic and the copy drifted.
//!
//! What is left here is what a *binding* is: [`binding::ELEMENT`] is the
//! `array<Element>` of that layout, [`binding::ALIVE`] is the flag array beside
//! it, and the group numbers below say where each is bound.
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
//! [`karakuri_ir::layout::generate_element_layout`] and
//! [`crate::l4::generate_l4`]'s signature, which takes the layout rather than
//! deriving its own.
//!
//! Six vertices per instance (`@builtin(vertex_index)` 0..6), no vertex
//! buffers: see the L4 module doc for the quad-expansion this exists for.

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
    /// In the input group beside the near side, because that is what it is: a
    /// second input edge, read and never written. It has its own struct rather
    /// than sharing `ElementIn`, since two sources need not emit the same
    /// attributes and each addresses its own buffer.
    ///
    /// **One, not one per declared slot.** A node takes one second geometry —
    /// the checker refuses a second `uses` — so this is a constant rather than
    /// a base an index is added to.
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

/// The camera edge, in the two shapes it has on the GPU.
///
/// **`CameraState` is the edge and `Camera` is what an L4 reads.** The six
/// numbers a camera *is* cross between the producer and the derivation;
/// `view_proj`, the ray basis and `depth_range` cross between the derivation and
/// every renderer. Both declarations are emitted verbatim into the engine's
/// `camera.wgsl` and into every generated L4, so a producer, the derivation and
/// a reader cannot disagree about the bytes — the same argument
/// [`counts::WGSL`] is here for.
///
/// **The state is a storage buffer and the derived form is a uniform**, which
/// is why only one of them is padded to a 16-byte multiple by hand. The state
/// packs its three scalars into the padding after its three `vec3`s, which is
/// the whole of why it is 48 bytes rather than 64.
pub mod camera {
    /// Byte size of `CameraState`.
    pub const STATE_SIZE: u64 = 48;
    /// Byte size of `Camera`.
    pub const SIZE: u64 = 144;

    /// What a producer writes: the host, from a `camera` record, or an L3's
    /// compute pass. **Read-only everywhere else** — a renderer never sees it.
    ///
    /// **`look_at` is `target` under another name**, because `target` is a
    /// reserved word in WGSL. Everything outside a shader — the IR, the host
    /// struct, this document — calls it `target`; renaming it here is cheaper
    /// than renaming it everywhere for one grammar's sake.
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

    /// What the derivation writes and every L4 reads.
    ///
    /// `right` and `up` arrive **pre-scaled** by the field of view and the
    /// aspect ratio, so a marching fragment's ray is an interpolation and a
    /// normalize rather than a projection — see `karakuri_engine::camera::Basis`.
    /// `depth_range` is `(near, 1 / (far - near))`, so the shader multiplies.
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
    pub const SIZE: u64 = 20;
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
    beats: f32,
};
";
}

/// Writes `struct Element { ... };` for `layout`. Shared by [`crate::l1`]
/// and [`crate::l4`] so the two crate-internal call sites can never drift —
/// the whole point of L4 taking an [`ElementLayout`] instead of deriving one
/// from `consumes` is that this text has to be byte-identical between the
/// two, since they address the same physical buffer.
pub fn write_element_struct(out: &mut String, layout: &ElementLayout) {
    write_element_struct_named(out, "Element", layout);
}

/// The same, under a chosen name. An L2 addresses **two** element buffers of
/// different shapes — what reached it and what it writes — so it needs two
/// structs in one module, and neither can be called `Element` without the other
/// being called something else. See [`crate::l2`].
pub fn write_element_struct_named(out: &mut String, name: &str, layout: &ElementLayout) {
    out.push_str(&format!("struct {name} {{\n"));
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

/// The WGSL spelling of a **field's** `param`, in a caller's uniform, under the
/// slot that reached it.
///
/// **A different prefix, and that is what makes a collision impossible.** A
/// field's body is spliced into its caller's shader and reads its params out of
/// the caller's uniform, so the two sets of names share one struct — and a
/// renderer declaring `exposure` beside a field declaring `exposure` would
/// otherwise be one field with two meanings. Prefixing them apart costs
/// nothing and removes the refusal that would otherwise have to exist, which
/// is the better of the two: `--param Field:0:exposure` and
/// `--param L4:0:exposure` name different things and both work.
///
/// **The slot is in the prefix as well**, so that two fields in one caller are
/// two sets of values rather than one — a shape's `radius` and a cutter's
/// `radius` are different numbers, and a caller holding one name for both would
/// drive them together with no way to say so.
pub fn mangle_field_param(slot: &str, name: &str) -> String {
    format!("field_{slot}_{name}")
}

/// The **semantic** name of a field's `param`, which is what the engine looks a
/// uniform field up by.
///
/// **Not the WGSL spelling**, and the difference is a defect this had. A caller
/// declaring `param field_radius` beside a field declaring `param radius` gave
/// two uniform fields with the semantic name `field_radius`; the packer finds by
/// name and takes the first, so the second was never written and its assertion
/// took the render thread down. The prefix makes the *WGSL* namespace safe and
/// says nothing about this one.
///
/// The separator is a character no `.kir` identifier can contain, so this name
/// cannot collide with any declared one however it is spelled — and it
/// separates the slot from the param for the same reason, so that the engine
/// can take a key apart again without guessing where one name ends.
pub fn field_param_key(slot: &str, name: &str) -> String {
    format!("field\u{1}{slot}\u{1}{name}")
}

/// The WGSL spelling of a **Source slot's** identity, in the uniform of the
/// procedure that declared it.
///
/// A prefix of its own, on [`mangle_param`]'s terms: the slot name is `.kir`
/// text, so `uses param_x : Source` beside `param x : float` would otherwise be
/// two fields of one WGSL name. A different prefix from a param's and from a
/// field param's purely so a human reading generated WGSL can tell at a glance
/// which kind of declaration a name came from.
pub fn mangle_source_slot(slot: &str) -> String {
    format!("source_{slot}")
}

/// The **semantic** name of a Source slot's identity, which is what the engine
/// looks the uniform field up by — [`field_param_key`]'s counterpart, and its
/// reasoning verbatim.
///
/// The separator is a character no `.kir` identifier can contain, so a slot
/// called `only` cannot collide with a param a procedure happened to call
/// `source_only`, and the engine can take the key apart again to get the slot
/// back without guessing where the prefix ends.
pub fn source_slot_key(slot: &str) -> String {
    format!("source\u{1}{slot}")
}

/// The complete field order and size of a generated uniform struct. This is
/// the other half of the "byte offsets" contract alongside
/// [`karakuri_ir::layout::ElementSlot`] —
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

/// WGSL uniform-address-space `(align, size)` for the scalar/vector/matrix
/// types the uniform structs this crate emits ever use. Not a general WGSL
/// layout function — only the types [`crate::l1`] and [`crate::l4`] put in a
/// uniform struct appear here.
///
/// **A second function beside [`karakuri_ir::layout::StorageElemTy`], but not a
/// second table.** What justifies the function is its key and its coverage: a
/// uniform field's type arrives here already spelled as a `&str` — from
/// [`crate::ty::wgsl_ty`] or from a literal this crate chose — and three of the
/// types it must place (`i32`, `vec4<f32>`, `mat4x4<f32>`) are ones no element
/// ever holds, so that enum has no variant for them. Everything it *does* have
/// a variant for is asked of it rather than restated: WGSL's `AlignOf` and
/// `SizeOf` for scalars and vectors do not vary by address space, so a `vec3`
/// is the same 16/12 here that it is in an element, and a copy of that row
/// could only ever be a chance to disagree with it — the drift shape that
/// moving the element rules into `karakuri-ir` existed to end.
///
/// What the uniform address space genuinely adds is a rule about *structs and
/// arrays*, not about scalars: align 16, size a multiple of 16. That rule is
/// not in this function at all — it is [`UniformLayoutBuilder::finish`]'s
/// `align_up(_, 16)`, and the rounding it uses is [`align_up`] from the same
/// module below, imported rather than kept as a private twin here.
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

    /// Finishes the layout, padding to a 16-byte multiple. Returns the
    /// layout plus the number of trailing `f32` pad slots to declare in
    /// WGSL — see [`write_uniform_struct`] for how those are spelled
    /// (omitted from `fields` either way: it is wire padding, not a value
    /// the engine ever sets).
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
            assert!(
                !out.contains("array<f32"),
                "param_count={param_count}:\n{out}"
            );
        }
    }

    /// **One table, asked twice.** Every type [`StorageElemTy`] has a variant
    /// for must place identically in a uniform struct and in an element,
    /// because WGSL's `AlignOf`/`SizeOf` for scalars and vectors do not depend
    /// on the address space — so a row of [`align_size`] that answered
    /// differently from the element table would not be a second address
    /// space's rule, it would be one of the two being wrong. This asserts the
    /// delegation is still in place: a row copied back into the match below
    /// with a number of its own fails here, which is the only place such a
    /// copy is visible at all.
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
