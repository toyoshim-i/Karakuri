//! **How large a node's per-element buffers are, decided in one place.**
//!
//! Two callers per figure and no third: the constructor that *allocates* the
//! buffer, and [`crate::set::Plan::element_storage`], which answers the same
//! question with no device in hand. Nothing here is a copy of anything — that
//! is the whole point of the module existing rather than the arithmetic
//! sitting beside each `create_buffer` call.
//!
//! **The terms are not guessable from a procedure's text**, which is why a
//! reporter that re-derived them keeps coming out wrong: an L1 pays for two
//! directions of everything, the liveness flag is a separate array rather than
//! a field of the element struct, a compacted procedure pays for a destination
//! index its text never mentions, and an amplifier pays at the count it makes
//! rather than at the Set's. The stage-4 `bytes/element` figure was exactly
//! that second derivation — 96 bytes published against 312 allocated — and it
//! was withdrawn rather than corrected; `docs/roadmap.md`, M4.
//!
//! **What is counted is [`crate::set::ElementStorage`]'s question and not
//! this module's.** One entry per element is the rule there; the counts block,
//! the uniform block and the scan's block-sum pyramid are outside it. What is
//! decided here is how large the buffers that *are* counted come out.
//!
//! **Nothing here reads a device.** Every input is a capacity, an element
//! stride and a predicate the generator already answered, so the same figures
//! are available before an adapter exists — which is what lets a saved Set be
//! costed without being built.

use karakuri_ir::layout::ALIVE_BYTES;

use crate::set::ElementStorage;

/// One `u32` per element, which is what the compaction scan writes into it —
/// `crate::compaction`, "Where the buffers come from".
///
/// A named function rather than a `* 4` beside the allocation because the same
/// number is needed to *report* the buffer, and the scan's own module doc
/// already treats this size as something another crate's claims rest on: it is
/// a whole multiple of capacity, which is what keeps
/// [`ElementStorage::per_element`] an exact division.
pub(crate) fn dest_bytes(capacity: u32) -> u64 {
    u64::from(capacity) * u64::from(DEST_INDEX_BYTES)
}

/// A destination index is a `u32` and there is nothing beside it to align
/// against — the same reasoning [`ALIVE_BYTES`] is 4 for.
const DEST_INDEX_BYTES: u32 = 4;

/// **What one L1 node holds per element**: two directions of the element
/// buffer, two of the alive array, and a destination index where the live set
/// can change.
///
/// Built from what the generator already decided, so a caller cannot size a
/// buffer against one stride and report another.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SimulationStorage {
    capacity: u32,
    element: u64,
    alive: u64,
    dest: Option<u64>,
}

impl SimulationStorage {
    /// `stride` is `L1Shader::element_layout.stride` and `compacted` is
    /// `L1Shader::compacted` — whether the procedure has a `spawn` block or
    /// calls `kill()` anywhere.
    ///
    /// **`compacted` decides two things at once and that is deliberate**: that
    /// a `Compaction` is built, and that its destination index is charged for.
    /// `Simulation::build` reaches both through [`SimulationStorage::dest_buffer`],
    /// so a node cannot end up owning a scan it was not charged for or being
    /// charged for one it does not own.
    pub(crate) fn of(capacity: u32, stride: u32, compacted: bool) -> SimulationStorage {
        SimulationStorage {
            capacity,
            element: u64::from(capacity) * u64::from(stride),
            alive: u64::from(capacity) * u64::from(ALIVE_BYTES),
            dest: compacted.then(|| dest_bytes(capacity)),
        }
    }

    /// One direction's element buffer. **There are two of this size**, and the
    /// doubling lives in [`SimulationStorage::total`] rather than here: what
    /// allocates wants one buffer's size and what reports wants the pair's, and
    /// a single number serving both is how one of them ends up halved.
    pub(crate) fn element_buffer(self) -> u64 {
        self.element
    }

    /// One direction's alive array, on the same terms.
    pub(crate) fn alive_buffer(self) -> u64 {
        self.alive
    }

    /// The compaction scan's destination index, or `None` for a procedure whose
    /// live set cannot change — which has no scan at all rather than an unused
    /// one.
    pub(crate) fn dest_buffer(self) -> Option<u64> {
        self.dest
    }

    /// **What a built node of this shape reports**, and what a Set that has not
    /// been built can be told.
    ///
    /// `Simulation::element_storage` does *not* call this: it sums
    /// `wgpu::Buffer::size()` over the buffers it actually created, which is
    /// what makes the two figures independent enough for a test to be worth
    /// writing. They agree because the allocation is sized from the same
    /// struct, and `wgpu::Buffer::size()` is documented to return exactly the
    /// size a buffer was created with — see the `mod gpu` test in
    /// `tests/storage.rs` that asserts it for a real Set.
    pub(crate) fn total(self) -> ElementStorage {
        ElementStorage {
            bytes: 2 * self.element + 2 * self.alive + self.dest.unwrap_or(0),
            capacity: self.capacity,
        }
    }
}

/// **What one L2 node holds per element**: one element buffer at the chain's
/// stride, and an alive array only where it amplifies.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeformStorage {
    capacity: u32,
    elements: u64,
    alive: Option<u64>,
}

impl DeformStorage {
    /// `out_capacity` is what reached this node times its own factor — **not
    /// the Set's capacity**, which is the input every per-procedure estimate of
    /// this figure has lacked. `stride` is `L2Shader::element_layout.stride`,
    /// which carries everything upstream emitted and not just this procedure's
    /// `emit`.
    ///
    /// `amplifies` is `L2Shader::amplify.is_some()`. Asked as a predicate
    /// rather than derived from the factor, on the same terms as
    /// `Deform::amplifies`: what decides the alive array is whose buffers the
    /// chain is on, and a factor of one arriving here would be a checker
    /// question rather than an allocation one.
    pub(crate) fn of(out_capacity: u32, stride: u32, amplifies: bool) -> DeformStorage {
        DeformStorage {
            capacity: out_capacity,
            elements: u64::from(out_capacity) * u64::from(stride),
            alive: amplifies.then(|| u64::from(out_capacity) * u64::from(ALIVE_BYTES)),
        }
    }

    /// The one element buffer. **Also what the device limit is checked
    /// against** — `Deform::build` compares this against
    /// `max_storage_buffer_binding_size`, so the number refused and the number
    /// allocated are one expression.
    pub(crate) fn element_buffer(self) -> u64 {
        self.elements
    }

    /// The alive array an amplifier owns, or `None` for a node that emits the
    /// elements that reached it under the flags they arrived with and shares
    /// the array they are already in.
    pub(crate) fn alive_buffer(self) -> Option<u64> {
        self.alive
    }

    /// What a built node of this shape reports — see
    /// [`SimulationStorage::total`] for why `Deform::element_storage` reads its
    /// buffers instead of calling this.
    pub(crate) fn total(self) -> ElementStorage {
        ElementStorage {
            bytes: self.elements + self.alive.unwrap_or(0),
            capacity: self.capacity,
        }
    }
}
