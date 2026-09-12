//! Storage buffer sizing for simulation and deformation nodes.
//!
//! Provides exact memory calculations for GPU element buffers, liveness arrays,
//! and compaction indices without requiring a live GPU device.

use karakuri_ir::layout::ALIVE_BYTES;

use crate::set::ElementStorage;

/// Computes the destination buffer size in bytes for compaction scans.
pub(crate) fn dest_bytes(capacity: u32) -> u64 {
    u64::from(capacity) * u64::from(DEST_INDEX_BYTES)
}

/// Byte size of each destination index in the compaction scan buffer.
const DEST_INDEX_BYTES: u32 = 4;

/// Storage footprint for an L1 simulation node.
///
/// Accounts for ping-pong element and alive buffers, plus optional compaction indices.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SimulationStorage {
    capacity: u32,
    element: u64,
    alive: u64,
    dest: Option<u64>,
}

impl SimulationStorage {
    /// Computes buffer sizes for an L1 node with the given capacity, stride, and compaction state.
    pub(crate) fn of(capacity: u32, stride: u32, compacted: bool) -> SimulationStorage {
        SimulationStorage {
            capacity,
            element: u64::from(capacity) * u64::from(stride),
            alive: u64::from(capacity) * u64::from(ALIVE_BYTES),
            dest: compacted.then(|| dest_bytes(capacity)),
        }
    }

    /// Returns the size in bytes of one element buffer.
    pub(crate) fn element_buffer(self) -> u64 {
        self.element
    }

    /// Returns the size in bytes of one alive buffer.
    pub(crate) fn alive_buffer(self) -> u64 {
        self.alive
    }

    /// Returns the size in bytes of the compaction destination buffer, if compacted.
    pub(crate) fn dest_buffer(self) -> Option<u64> {
        self.dest
    }

    /// Returns the total element storage metrics for this node configuration.
    pub(crate) fn total(self) -> ElementStorage {
        ElementStorage {
            bytes: 2 * self.element + 2 * self.alive + self.dest.unwrap_or(0),
            capacity: self.capacity,
        }
    }
}

/// Storage footprint for an L2 deformation node.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeformStorage {
    capacity: u32,
    elements: u64,
    alive: Option<u64>,
}

impl DeformStorage {
    /// Computes buffer sizes for an L2 node with the given output capacity, stride, and amplification.
    pub(crate) fn of(out_capacity: u32, stride: u32, amplifies: bool) -> DeformStorage {
        DeformStorage {
            capacity: out_capacity,
            elements: u64::from(out_capacity) * u64::from(stride),
            alive: amplifies.then(|| u64::from(out_capacity) * u64::from(ALIVE_BYTES)),
        }
    }

    /// Returns the size in bytes of the element output buffer.
    pub(crate) fn element_buffer(self) -> u64 {
        self.elements
    }

    /// Returns the size in bytes of the allocated alive buffer if amplifying, or `None`.
    pub(crate) fn alive_buffer(self) -> Option<u64> {
        self.alive
    }

    /// Returns the total element storage metrics for this deformation node.
    pub(crate) fn total(self) -> ElementStorage {
        ElementStorage {
            bytes: self.elements + self.alive.unwrap_or(0),
            capacity: self.capacity,
        }
    }
}
