//! Where per-element state lands in a buffer.
//!
//! **The placement rules, not the binding contract.** Which group and binding
//! number a buffer is bound at, and what WGSL text declares it, belong to
//! `karakuri-codegen::layout` and are still there. What is here is the one
//! arithmetic that decides how many bytes an element occupies and at what
//! offset within them each of its slots begins — the thing the lowering writes
//! a struct from and the engine sizes a buffer by.
//!
//! **It lives in the crate at the bottom of the dependency graph so that
//! nothing which needs it has to keep a copy.** It was in `karakuri-codegen`,
//! which the estimator in [`crate::cost`] cannot call — `karakuri-codegen`
//! depends on this crate and not the reverse — so stage 4 carried its own copy,
//! the two drifted the moment the real one changed, and `drift_shell` was
//! reported at 192 bytes an element where its buffers are 104. Stage 4 has
//! since stopped asking the question at all, for a reason of its own that
//! [`crate::cost`] states, and the argument for this address survives it: what
//! an element struct contains is decided by a procedure's `emit`, by [`Attr`]'s
//! own widths and by which derivations are stored, and those are all facts of
//! this crate. A placement rule kept anywhere above them is a second
//! description of them, waiting to disagree.
//!
//! # Per-element state: one buffer per direction, not one per attribute
//!
//! Every emitted attribute used to get its own pair of storage buffers, so a
//! procedure emitting three attributes bound 12 storage buffers in its
//! compute stage (3 synthetic + 3 declared, times prev and next) — more than
//! the WebGPU default limit of 8, and twice the downlevel default of 4. All
//! per-element state that lives in the *same* buffer as everything else of
//! its own direction packs into one generated `Element` struct instead: one
//! slot per entry (`seed`, `birth_frac`, then `emit` in declaration order),
//! **each at its own width and at the offset WGSL's layout rules give
//! it**. A `vec3` is 16-byte aligned and 12 bytes long, so a scalar declared
//! after one lands in the four bytes it leaves: `position, size` is one
//! 16-byte block rather than two.
//!
//! `alive` is the one exception: it leaves the struct entirely and becomes
//! its own tight `array<u32>` buffer, [`ALIVE_BYTES`] per element, because the
//! compaction scan (`crates/karakuri-engine/src/shaders/scan.wgsl`) reads
//! alive flags as a dense array with no stride arithmetic. Packing it into
//! `Element` would mean the scan needs to know the struct's per-procedure
//! size just to skip to the next flag; a dense array needs nothing but the
//! element count.
//!
//! # Why a crate that emits no WGSL spells WGSL type names
//!
//! [`StorageElemTy::align`] and [`StorageElemTy::size`] **are** WGSL's table,
//! not a table this project chose — a crate that owns them already knows WGSL's
//! type system, and [`StorageElemTy::wgsl_name`] beside them is the same fact
//! under its own spelling. Splitting the spelling off into the crate that emits
//! the text would put one type's three properties in two places, which is the
//! shape of defect this module exists to end. `karakuri-ir` already hands out
//! WGSL identifiers besides: every [`Attr::name`] is one, and the generated
//! struct's field names are exactly those strings.

use crate::ast::{Attr, Derivation, Ty};

/// Bytes per element in the alive buffer.
///
/// **A dense `array<u32>` and not a slot in `Element`**, so this is a stride of
/// its own rather than an offset into one — see the module doc for why the
/// compaction scan cannot afford the flag to live in a struct whose size
/// depends on which procedure wrote it. It is `4` because the flag is a `u32`
/// and there is nothing beside it to align against.
pub const ALIVE_BYTES: u32 = 4;

/// Rounds `offset` up to the next multiple of `align`.
///
/// **The one rounding in this project's layouts**, shared by the element
/// placement here and by `karakuri-codegen`'s uniform-struct builder rather
/// than written twice — the two are the same rule out of the same
/// specification, and a private twin in each crate is how they would come to
/// differ.
pub fn align_up(offset: u32, align: u32) -> u32 {
    offset.div_ceil(align) * align
}

/// The WGSL element type an `Element` struct field is declared with — the
/// attribute's own width, and the alignment and size WGSL gives it.
///
/// **These are WGSL's numbers, not this crate's.** A layout that invented its
/// own would be a second place for the same fact, and the two would disagree
/// the first time a field type was added: the host writes bytes at
/// [`ElementSlot::offset`] and the shader reads them through the struct, so a
/// disagreement is not a compile error anywhere — it is an element reading the
/// middle of the element before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageElemTy {
    U32,
    F32,
    Vec2F32,
    Vec3F32,
}

impl StorageElemTy {
    /// Every variant, so that a lookup *by* WGSL spelling can be a search over
    /// this list instead of a second spelling table written backwards beside
    /// [`StorageElemTy::wgsl_name`].
    ///
    /// **A variant missing from here fails loudly rather than quietly.**
    /// [`StorageElemTy::from_wgsl_name`] answers `None` for it, and the caller
    /// that asks — `karakuri-codegen`'s uniform table — panics naming the type
    /// it could not place. That is the affordable failure: the unaffordable one
    /// is a spelling that resolves to the wrong width, because nothing downstream
    /// of a wrong width is a compile error.
    pub const ALL: [StorageElemTy; 4] = [
        StorageElemTy::U32,
        StorageElemTy::F32,
        StorageElemTy::Vec2F32,
        StorageElemTy::Vec3F32,
    ];

    /// The variant a WGSL spelling names, or `None` for a type no element ever
    /// holds and this enum therefore has no variant for.
    ///
    /// **The inverse of [`StorageElemTy::wgsl_name`], computed and not written**,
    /// which is what keeps it from becoming the third copy of the same
    /// correspondence. It exists because `karakuri-codegen` reaches this table
    /// through a `&str`: a uniform field's type arrives already spelled, from
    /// `karakuri_ir::Ty` or from a literal that crate chose, and asking here by
    /// spelling is what lets its uniform layout share these numbers rather than
    /// restate them.
    pub fn from_wgsl_name(wgsl_ty: &str) -> Option<StorageElemTy> {
        StorageElemTy::ALL
            .into_iter()
            .find(|t| t.wgsl_name() == wgsl_ty)
    }

    pub fn wgsl_name(self) -> &'static str {
        match self {
            StorageElemTy::U32 => "u32",
            StorageElemTy::F32 => "f32",
            StorageElemTy::Vec2F32 => "vec2<f32>",
            StorageElemTy::Vec3F32 => "vec3<f32>",
        }
    }

    /// **`vec3` is the one that surprises**: 16-byte aligned and 12 bytes long,
    /// so the four bytes after one are addressable and a scalar declared next
    /// lands in them for free. That is where most of what this layout saves
    /// comes from — `position` followed by `size` costs 16 bytes, not 32.
    pub fn align(self) -> u32 {
        match self {
            StorageElemTy::U32 | StorageElemTy::F32 => 4,
            StorageElemTy::Vec2F32 => 8,
            StorageElemTy::Vec3F32 => 16,
        }
    }

    pub fn size(self) -> u32 {
        match self {
            StorageElemTy::U32 | StorageElemTy::F32 => 4,
            StorageElemTy::Vec2F32 => 8,
            StorageElemTy::Vec3F32 => 12,
        }
    }
}

/// One field of the generated `Element` struct, at its own width.
///
/// **This was every field padded to a `vec4`**, so that the stride was
/// `(2 + emit.len()) * 16` with no per-attribute case analysis. The case
/// analysis is four lines of [`StorageElemTy`] and the padding was **39% of
/// every element buffer** across the geometries this repository ships — and an
/// amplifying stage multiplies exactly that number, so the same fraction comes
/// off the largest allocation in the system.
///
/// What the uniform width bought was that nothing had to know WGSL's layout
/// rules. Something does now, and the risk is real: the host writes bytes at
/// [`ElementSlot::offset`] and the shader reads them through the struct, so a
/// disagreement is not a compile error anywhere — it is an element reading the
/// middle of the element before it. That is why the align/size table lives in
/// one place and is asserted against a real WGSL module: the naga tests in
/// `karakuri-codegen` compile the emitted struct, read back the member offsets
/// and array stride *naga* computed from its own rules, and compare them
/// against [`ElementSlot::offset`] and [`ElementLayout::stride`].
///
/// **Validating the module is a weaker check and would never have caught it.**
/// Nothing here emits an `@offset` attribute, so a front end handed a wrong
/// table derives its offsets from the same declarations, arrives at its own
/// answer, and agrees with itself. Only asking it what that answer *was* can
/// disagree.
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
    /// Byte offset of this field within one `Element` entry, by WGSL's own
    /// placement rules — published rather than left for a caller to recompute,
    /// same reasoning as `karakuri_codegen::layout::UniformField::offset`, and
    /// now load-bearing rather than merely convenient: it is no longer
    /// `index * 16` and there is no second way to arrive at it.
    pub offset: u32,
}

/// The ordered slots of a generated `Element` struct, plus the struct's
/// byte stride. This is the contract `karakuri-engine` drives: it sizes the
/// element buffer as `capacity * stride`, and [`ElementLayout::offset_of`]
/// is how it finds where a given slot's bytes land when it needs to write
/// one directly (initializing `seed` and `birth_frac` for a spawn-less
/// procedure — see `karakuri-engine`'s `node::Simulation::initialize`).
#[derive(Debug, Clone, PartialEq)]
pub struct ElementLayout {
    /// `seed`, then `birth_frac`, then the engine-written slots this position
    /// carries, then `emit` in declaration order — see
    /// [`generate_element_layout`].
    pub slots: Vec<ElementSlot>,
    /// The array stride: the struct's size rounded up to its own alignment,
    /// which is the rule WGSL applies to `array<Element>`. **Not** a function
    /// of the slot count.
    pub stride: u32,
    /// **Attributes readable here that have no slot**, synthesised at the read
    /// site from one that does.
    ///
    /// This is what makes an `ElementLayout` the *contract* rather than a byte
    /// layout: two of the things a consumer may name are not fields, and a
    /// reader that only had `slots` would have to be told about them
    /// separately — which is a second place for one fact. Everything that
    /// lowers an attribute read asks this list first.
    pub derived: Vec<Attr>,
}

impl ElementLayout {
    /// Whether the slot named `name` exists — for the engine-written ones,
    /// which carry no [`Attr`], where asking by [`Attr`] is not possible.
    pub fn has_slot(&self, name: &str) -> bool {
        self.slots.iter().any(|s| s.name == name)
    }

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
    // `karakuri-ir/src/ast.rs`. None of them is ever uint; only the synthetic
    // `seed` and `copy` slots are.
    match attr.ty() {
        Ty::Float => StorageElemTy::F32,
        Ty::Vec2 => StorageElemTy::Vec2F32,
        Ty::Vec3 => StorageElemTy::Vec3F32,
        other => unreachable!(
            "attribute {} has non-float-family type {:?}",
            attr.name(),
            other
        ),
    }
}

/// The slots the engine writes and no procedure declares, beyond the two every
/// element has.
///
/// **Conditional, because each one costs its own width on every element of
/// every Set that has it** — four bytes for `copy`, plus whatever alignment it
/// drags behind it. `seed` and `birth_frac` are unconditional because identity
/// and spawn timing are properties of an element as such; what is here is a
/// property of what happened *upstream*, so a chain that never amplified
/// carries no `copy` and a Set that pays for one is a Set that has one.
///
/// One struct rather than a parameter per slot: `docs/roadmap.md` has two more
/// of these coming — `source` from multiple L1 sources — and three independent
/// booleans threaded through the same call sites is three places to forget one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Synthetic {
    /// `copy`: which copy of its parent this element is, from an amplifying L2
    /// upstream. See [`crate::Ambient::Copy`].
    pub copy: bool,
}

impl Synthetic {
    /// What an L1 writes: neither, always. Nothing has amplified above the node
    /// that makes the elements.
    pub const NONE: Synthetic = Synthetic { copy: false };
}

/// The companion flag a *stored* derivation needs: whether this element has
/// lived a whole step, and so whether the slot beside it holds a real
/// difference or the distance from wherever `spawn` put it.
pub const LIVED: &str = "velocity_lived";

/// The slot a derivation rule needs on the element, and what it holds.
///
/// **Named here rather than beside the rule in [`Attr::derivation`]**, because
/// a name is a WGSL identifier that appears in a generated struct and at every
/// read of it, and that is a placement question rather than a semantic one: the
/// name and the offset it lands at are decided together or not at all. The rule
/// says an attribute is a frame difference; this says which four or twelve
/// bytes hold it and what the shader calls them.
pub fn derivation_slot(attr: Attr) -> Option<(&'static str, Option<Attr>)> {
    match attr.derivation()? {
        // Holds the spawn instant; `age` itself is the subtraction at the read
        // site and has no slot.
        Derivation::SinceBirth => Some(("birth_t", None)),
        // Holds `velocity` itself, written by the L1 against the step it
        // already has. A reader sees an ordinary attribute — see
        // `Derivation::is_stored`.
        Derivation::FrameDifference(Attr::Position) => Some(("velocity", Some(Attr::Velocity))),
        // The rule table has one source today. A second would be a second slot
        // and a second name, which is a decision rather than a line to add.
        Derivation::FrameDifference(_) => None,
    }
}

/// Builds the `Element` struct layout for a procedure's `emit` list:
/// `seed` first, then `birth_frac`, then whichever of [`Synthetic`] this
/// position carries, then `emit` in declaration order. Fixed order because it
/// is the shape of a struct in WGSL text, not a set of independently
/// addressable bindings — unlike the old per-attribute binding numbers, there
/// is no freedom to reorder without changing what every `prev[i].<field>`
/// access compiles to.
///
/// **The synthetic slots come before the declared ones** so that the engine's
/// direct writes — [`ElementLayout::offset_of`] for `seed` and `birth_frac` —
/// land at offsets that do not move when a procedure declares one more
/// attribute. It costs the property that two layouts in one chain share a
/// prefix past the point an amplifier widened it, which nothing relies on:
/// every node is compiled against the exact layout it was handed.
///
/// L4 must be generated against the exact [`ElementLayout`] this returns for
/// the node it reads, since it addresses the same physical buffer that node
/// wrote — see `karakuri_codegen::l4::generate_l4`.
pub fn generate_element_layout(
    emit: &[Attr],
    synthetic: Synthetic,
    derived: &[Attr],
) -> ElementLayout {
    let seed = ("seed", None, StorageElemTy::U32);
    let birth_frac = ("birth_frac", None, StorageElemTy::F32);
    let copy = synthetic.copy.then_some(("copy", None, StorageElemTy::U32));
    // **A derivation's source slot, allocated because something asked for the
    // attribute it feeds.** This is the whole of what "the layout is the
    // compiled form of the contract" means in practice: `emit` no longer
    // decides the struct on its own, and a slot can exist that no procedure
    // named. It stays conditional for the reason `copy` is — sixteen bytes on
    // every element of every Set is what unconditional costs.
    let sources: Vec<(&'static str, Option<Attr>, StorageElemTy)> = derived
        .iter()
        .filter_map(|&attr| derivation_slot(attr))
        .flat_map(|(name, holds)| {
            let held = holds.map_or(StorageElemTy::F32, attr_elem_ty);
            // **The flag is its own field now, and it costs nothing.** A stored
            // derivation needs to say whether this element has lived a whole
            // step yet — an element's first update has no previous position to
            // difference against, and a velocity computed from one is the whole
            // spawn distance over one `dt`. That flag used to live in the `.w`
            // of a padded `vec4`, which was free because the padding was there
            // anyway. It is still free: `velocity` is a `vec3` and a `f32`
            // declared after one lands in the four bytes WGSL leaves after it.
            let flag = holds
                .filter(|a| a.derivation().is_some_and(|d| d.is_stored()))
                .map(|_| (LIVED, None, StorageElemTy::F32));
            std::iter::once((name, holds, held)).chain(flag)
        })
        .collect();
    let declared = emit
        .iter()
        .map(|&attr| (attr.name(), Some(attr), attr_elem_ty(attr)));
    // **Declaration order, and WGSL's own placement rules over it.** No
    // reordering to pack tighter: an author's `emit` order is the order the
    // struct reads in, and a layout that sorted would make the offsets a
    // function of something nobody wrote. It packs well anyway, because the
    // interesting case is a `vec3` with a scalar after it — `position, size`
    // is one 16-byte block rather than two.
    let mut at = 0u32;
    let mut align = 4u32;
    let slots: Vec<ElementSlot> = std::iter::once(seed)
        .chain(std::iter::once(birth_frac))
        .chain(copy)
        .chain(sources)
        .chain(declared)
        .map(|(name, attr, elem_ty)| {
            let offset = align_up(at, elem_ty.align());
            at = offset + elem_ty.size();
            align = align.max(elem_ty.align());
            ElementSlot {
                name,
                attr,
                elem_ty,
                offset,
            }
        })
        .collect();
    // The array stride, which is the struct's size rounded up to its own
    // alignment — the rule WGSL applies to `array<Element>` and therefore the
    // one the host has to size a buffer by.
    let stride = align_up(at, align);
    // **Only the rules a reader has to do arithmetic for.** Where the engine
    // stores the attribute itself the slot above already carries it, and
    // listing it here as well would make `offers` true twice and `is_stored`
    // and `derived` disagree about the same attribute.
    let substituted = derived
        .iter()
        .copied()
        .filter(|a| a.derivation().is_some_and(|d| !d.is_stored()))
        .collect();
    ElementLayout {
        slots,
        stride,
        derived: substituted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_layout_puts_seed_and_birth_frac_first_at_fixed_offsets() {
        let layout = generate_element_layout(&[Attr::Position, Attr::Age], Synthetic::NONE, &[]);
        assert_eq!(layout.slots[0].name, "seed");
        assert_eq!(layout.slots[0].offset, 0);
        assert_eq!(layout.slots[1].name, "birth_frac");
        assert_eq!(layout.slots[1].offset, 4);
        assert_eq!(layout.slots[2].attr, Some(Attr::Position));
        assert_eq!(layout.slots[2].offset, 16);
        // And `age` is a `f32` in the four bytes `position` leaves.
        assert_eq!(layout.slots[3].attr, Some(Attr::Age));
        assert_eq!(layout.slots[3].offset, 28);
    }

    #[test]
    fn element_layout_stride_follows_wgsl_placement() {
        // Two scalars and nothing else: 8 bytes, aligned to 4.
        assert_eq!(generate_element_layout(&[], Synthetic::NONE, &[]).stride, 8);
        assert_eq!(
            generate_element_layout(&[Attr::Position], Synthetic::NONE, &[]).stride,
            32
        );
        assert_eq!(
            generate_element_layout(
                &[Attr::Position, Attr::Velocity, Attr::Age],
                Synthetic::NONE,
                &[]
            )
            .stride,
            // seed(0) birth_frac(4) position(16..28) velocity(32..44)
            // age(44..48) — 80 bytes under the padded layout, 48 here.
            48
        );
    }

    /// **`copy` is allocated only where something upstream amplified**, which
    /// is the whole reason [`Synthetic`] exists as a parameter rather than the
    /// slot being unconditional like the two above it — and it is now free
    /// besides, because `seed` and `birth_frac` leave eight bytes of the first
    /// block unused and `copy` lands in four of them.
    #[test]
    fn the_copy_slot_is_allocated_only_when_something_amplified() {
        let plain = generate_element_layout(&[Attr::Position], Synthetic::NONE, &[]);
        assert!(!plain.slots.iter().any(|s| s.name == "copy"), "{plain:?}");
        assert_eq!(plain.stride, 32, "u32, f32, then a vec3 at 16");

        let amplified = generate_element_layout(&[Attr::Position], Synthetic { copy: true }, &[]);
        assert_eq!(amplified.slots[2].name, "copy");
        assert_eq!(amplified.slots[2].attr, None, "no procedure declares it");
        assert_eq!(amplified.offset_of("copy"), 8);
        assert_eq!(amplified.stride, 32, "the copy index cost nothing");
        // **Before the declared attributes, with the other engine-written
        // slots**, so that the two offsets the engine writes directly do not
        // move when a procedure declares one more attribute.
        assert_eq!(amplified.offset_of("seed"), 0);
        assert_eq!(amplified.offset_of("birth_frac"), 4);
        assert_eq!(amplified.offset_of("position"), 16);
    }

    #[test]
    fn offset_of_finds_the_synthetic_slots() {
        let layout = generate_element_layout(&[Attr::Position], Synthetic::NONE, &[]);
        assert_eq!(layout.offset_of("seed"), 0);
        assert_eq!(layout.offset_of("birth_frac"), 4);
        assert_eq!(layout.offset_of("position"), 16);
    }

    /// **A slot is its attribute's own width, and the offsets are WGSL's.**
    /// Every field used to be a padded `vec4` so that the stride was
    /// `slots.len() * 16` with no case analysis. The case analysis is four
    /// lines and the padding was a fifth of every element buffer in the
    /// system — and an amplifying stage multiplies exactly that number.
    #[test]
    fn a_scalar_after_a_vec3_lands_in_the_padding_that_vec3_leaves() {
        // `position` is 12 bytes at 16-byte alignment, so 28..32 is
        // addressable and `size` is placed there rather than at 32.
        let layout = generate_element_layout(&[Attr::Position, Attr::Size], Synthetic::NONE, &[]);
        assert_eq!(layout.offset_of("position"), 16);
        assert_eq!(layout.offset_of("size"), 28);
        assert_eq!(layout.stride, 32);

        // The same emit list under the old padded layout was four slots of
        // sixteen. This is the saving, stated as a number rather than as a
        // property.
        assert!(layout.stride < 4 * 16);
    }

    /// The stride is the struct's size rounded up to its own alignment, which
    /// is the rule WGSL applies to `array<Element>` — so the host sizing a
    /// buffer as `capacity * stride` and the shader indexing it agree.
    #[test]
    fn the_stride_is_the_struct_alignment_not_the_last_field_end() {
        // seed(0..4), birth_frac(4..8), tint(16..28) — 28 rounded up to the
        // vec3's 16-byte alignment.
        let layout = generate_element_layout(&[Attr::Tint], Synthetic::NONE, &[]);
        assert_eq!(layout.offset_of("tint"), 16);
        assert_eq!(layout.stride, 32);

        // With no vector at all the alignment is 4 and nothing is rounded.
        let scalars = generate_element_layout(&[Attr::Size], Synthetic::NONE, &[]);
        assert_eq!(scalars.offset_of("size"), 8);
        assert_eq!(scalars.stride, 12);
    }
}
