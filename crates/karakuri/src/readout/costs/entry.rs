use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::time::Duration;

// ---------------------------------------------------------------------------
// What a still panel costs
// ---------------------------------------------------------------------------

/// Duration of window inactivity required before measuring idle panel performance (ADR-0164).
pub(crate) const STILL: Duration = Duration::from_secs(3);

/// Maximum capacity of the rolling frame cost sample buffer.
pub(crate) const SAMPLE: usize = 240;

/// Reference allocation baseline for egui UI passes on untouched panels (ADR-0164, ADR-0191).
#[allow(dead_code)]
pub(crate) const WRITTEN_ALLOCS: u64 = 1518;
#[allow(dead_code)]
pub(crate) const WRITTEN_KB: f64 = 1781.6;
#[allow(dead_code)]
pub(crate) const WRITTEN_ON: &str = "2026-08-31";

/// Maximum allowable ratio between measured allocations/time and baseline before reporting drift.
#[allow(dead_code)]
pub(crate) const DRIFT: f64 = 2.0;

/// Returns the drift factor if `measured` deviates from `written` baseline by more than [`DRIFT`].
#[allow(dead_code)]
pub(crate) fn drifted(measured: u64, written: u64) -> Option<f64> {
    let factor = measured.max(written) as f64 / measured.min(written).max(1) as f64;
    (factor > DRIFT).then_some(factor)
}

/// Returns the drift factor for duration metrics if `measured` deviates by more than [`DRIFT`].
#[allow(dead_code)]
pub(crate) fn drifted_ms(measured: f64, written: f64) -> Option<f64> {
    let factor = measured.max(written) / measured.min(written).max(f64::MIN_POSITIVE);
    (factor > DRIFT).then_some(factor)
}

/// The allocator, counting. Per thread, not per process — `wgpu` allocates on
/// threads of its own and a process-wide counter would attribute that to the
/// `egui` pass, which is the one number this exists to get right.
pub(crate) struct Counting;

thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
    static BYTES: Cell<u64> = const { Cell::new(0) };
}

/// `(allocations, bytes)` this thread has asked for since it started.
/// Reallocations count as one allocation of the new size, which overstates a
/// growing `Vec` and is the conservative direction.
pub(crate) fn counted() -> (u64, u64) {
    (
        ALLOCS.try_with(Cell::get).unwrap_or(0),
        BYTES.try_with(Cell::get).unwrap_or(0),
    )
}

pub(crate) fn count(size: usize) {
    let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
    let _ = BYTES.try_with(|c| c.set(c.get() + size as u64));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
        count(layout.size());
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: AllocLayout) -> *mut u8 {
        count(layout.size());
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: AllocLayout, new_size: usize) -> *mut u8 {
        count(new_size);
        System.realloc(ptr, layout, new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout) {
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
pub(crate) static ALLOCATOR: Counting = Counting;

/// CPU and GPU metrics measured for a single frame.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Cost {
    /// CPU duration of engine deck simulation and composite pass.
    pub(crate) engine: Duration,
    /// CPU duration of egui immediate-mode layout, widget construction, and tessellation.
    pub(crate) ui: Duration,
    /// CPU duration of buffer uploads and command encoder recording for the UI pass.
    pub(crate) paint: Duration,
    /// CPU duration for updating egui texture deltas (e.g. font atlas updates).
    pub(crate) textures: Duration,
    /// CPU duration for uploading tessellated vertex and index buffers to GPU memory.
    pub(crate) buffers: Duration,
    /// CPU duration for recording egui render pass command buffer.
    pub(crate) record: Duration,
    /// CPU duration for command buffer queue submission via `wgpu::Queue::submit`.
    pub(crate) submit: Duration,
    /// Duration blocked waiting for swapchain image acquisition (`get_current_texture`).
    #[allow(dead_code)]
    pub(crate) wait: Duration,
    /// Total wall-clock interval between successive frame redraw requests (`RedrawRequested`).
    pub(crate) period: Option<Duration>,
    /// Optional GPU execution duration measured by draining device queue during periodic audits.
    pub(crate) drained: Option<Duration>,
    /// Allocations during `ui` pass on this thread.
    pub(crate) allocs: u64,
    /// Total bytes allocated during `ui` pass on this thread.
    pub(crate) bytes: u64,
}

impl Cost {
    /// Total CPU time accounted for across engine composite, UI, and paint passes (`engine + ui + paint`).
    pub(crate) fn whole(&self) -> Duration {
        self.engine + self.ui + self.paint
    }

    /// Unaccounted wall-clock time in the frame period after subtracting swapchain wait and [`Cost::whole`].
    #[allow(dead_code)]
    pub(crate) fn elsewhere(&self) -> Option<Duration> {
        Some(
            self.period?
                .saturating_sub(self.wait)
                .saturating_sub(self.whole()),
        )
    }
}

/// Metric counters captured during periods of window inactivity (ADR-0164).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Still {
    pub(crate) frames: usize,
    pub(crate) allocs: u64,
    pub(crate) bytes: u64,
}
