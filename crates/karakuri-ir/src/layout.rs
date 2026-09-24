//! Memory layout computation for storage buffer element data.
//!
//! Computes field offsets and byte stride of generated `Element` structs
//! following WGSL storage buffer alignment and sizing rules.

use crate::ast::{Attr, Derivation, Ty};

/// Bytes per element in the alive buffer (`4` bytes for a dense `u32` array).
pub const ALIVE_BYTES: u32 = 4;

/// Rounds `offset` up to the next multiple of `align`.
pub fn align_up(offset: u32, align: u32) -> u32 {
    offset.div_ceil(align) * align
}

/// WGSL storage element type with corresponding alignment and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageElemTy {
    U32,
    F32,
    Vec2F32,
    Vec3F32,
}

impl StorageElemTy {
    /// All storage element type variants.
    pub const ALL: [StorageElemTy; 4] = [
        StorageElemTy::U32,
        StorageElemTy::F32,
        StorageElemTy::Vec2F32,
        StorageElemTy::Vec3F32,
    ];

    /// Returns the [`StorageElemTy`] corresponding to the given WGSL type name,
    /// or `None` if unrepresented in element storage.
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

    /// `vec3` is the one that surprises: 16-byte aligned and 12 bytes long, so the
    /// four bytes after one are addressable and a scalar declared next lands in
    /// them for free. That is where most of what this layout saves comes from —
    /// `position` followed by `size` costs 16 bytes, not 32.
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

/// A single field in the generated `Element` storage struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementSlot {
    /// Field name matching the WGSL identifier.
    pub name: &'static str,
    /// Corresponding attribute, or `None` for synthetic fields (`seed`, `birth_frac`, `copy`).
    pub attr: Option<Attr>,
    pub elem_ty: StorageElemTy,
    /// Byte offset of this field within the `Element` struct.
    pub offset: u32,
}

/// Ordered layout specification for generated `Element` storage buffer structs.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementLayout {
    /// Struct fields ordered by layout.
    pub slots: Vec<ElementSlot>,
    /// Array stride in bytes, aligned to the struct's required alignment.
    pub stride: u32,
    /// Attributes synthesised at the read site from available element slots.
    pub derived: Vec<Attr>,
}

impl ElementLayout {
    /// Whether the slot named `name` exists — for the engine-written ones, which
    /// carry no [`Attr`], where asking by [`Attr`] is not possible.
    pub fn has_slot(&self, name: &str) -> bool {
        self.slots.iter().any(|s| s.name == name)
    }

    /// The byte offset of the slot named `name` within one `Element` entry. Panics
    /// if no such slot exists — every caller asks for `seed` or `birth_frac`, which
    /// [`generate_element_layout`] always allocates.
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

/// Flags controlling inclusion of conditional synthetic slots in the `Element` struct.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Synthetic {
    /// Whether an upstream amplifying L2 node introduced a `copy` slot.
    pub copy: bool,
}

impl Synthetic {
    /// Configuration for standard unamplified L1 procedures.
    pub const NONE: Synthetic = Synthetic { copy: false };
}

/// Field name for the velocity lived initialization flag.
pub const LIVED: &str = "velocity_lived";

/// Returns the element slot name and attribute for a given attribute derivation, if any.
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

/// Builds the `Element` struct layout for a procedure: `seed`, `birth_frac`,
/// conditional synthetic slots, and declared `emit` attributes in order.
pub fn generate_element_layout(
    emit: &[Attr],
    synthetic: Synthetic,
    derived: &[Attr],
) -> ElementLayout {
    let seed = ("seed", None, StorageElemTy::U32);
    let birth_frac = ("birth_frac", None, StorageElemTy::F32);
    let copy = synthetic.copy.then_some(("copy", None, StorageElemTy::U32));
    // Allocate source slots for derived attributes.
    let sources: Vec<(&'static str, Option<Attr>, StorageElemTy)> = derived
        .iter()
        .filter_map(|&attr| derivation_slot(attr))
        .flat_map(|(name, holds)| {
            let held = holds.map_or(StorageElemTy::F32, attr_elem_ty);
            // Stored derivations include an initialization status flag.
            let flag = holds
                .filter(|a| a.derivation().is_some_and(|d| d.is_stored()))
                .map(|_| (LIVED, None, StorageElemTy::F32));
            std::iter::once((name, holds, held)).chain(flag)
        })
        .collect();
    let declared = emit
        .iter()
        .map(|&attr| (attr.name(), Some(attr), attr_elem_ty(attr)));
    // Place fields in declaration order following WGSL storage alignment rules.
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

    /// Tests that the copy slot is allocated only when synthetic copy is enabled.
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
        // Verify engine-written slot offsets.
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

    /// Tests that scalar fields pack into padding bytes left by preceding vec3 fields.
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

    /// The stride is the struct's size rounded up to its own alignment, which is
    /// the rule WGSL applies to `array<Element>` — so the host sizing a buffer as
    /// `capacity * stride` and the shader indexing it agree.
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
