//! The one property of the audio callback that cannot be argued, only counted.
//!
//! `analysis` and `tempo` both claim to allocate nothing after construction,
//! because the callback runs them and an allocation on a real-time thread is a
//! lock in disguise — the allocator's, taken behind your back, for as long as
//! it feels like. Every buffer is planned in `Analyzer::new` and `Tracker::new`
//! and nothing in the hot path grows; the claim is worth something only if
//! something checks it, since `rustfft`'s per-call path is not this crate's
//! code and the day one of these buffers gains a `push` nothing else would
//! notice.
//!
//! A counting global allocator, armed around exactly the work one callback
//! does. It has to be its own integration test rather than a `#[test]` in the
//! crate: a `#[global_allocator]` is per-binary, and installing a counting one
//! under the whole unit test suite would count every other test's allocations
//! on whatever thread happened to be running.
//!
//! Counting is on from the first call rather than after a warm-up, so a
//! first-use allocation inside the transform — which would land on the audio
//! thread the first time a real stream produced a block — is caught too.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use karakuri_audio::analysis::{Analyzer, BLOCK};
use karakuri_audio::tempo::Tracker;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // A `Vec` that grows shows up here rather than in `alloc`, which is the
        // shape this test is most likely to catch.
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// A deterministic broadband block: something with content in every bin, so no
/// branch inside the transform or the estimator is skipped for being lucky.
fn block() -> Vec<f32> {
    let mut x: u32 = 0x9e37_79b9;
    (0..BLOCK)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x as f32 / u32::MAX as f32) * 2.0 - 1.0
        })
        .collect()
}

#[test]
fn one_hop_of_callback_work_allocates_nothing() {
    let mut analyzer = Analyzer::new(48_000);
    let mut tracker = Tracker::new(analyzer.hop_seconds(), analyzer.window_lag());
    let block = block();

    // Enough hops to cover every path a callback takes: the window filling, a
    // re-measurement (one push in every `ESTIMATE_INTERVAL_SECONDS`), and the
    // extrapolations between them.
    COUNTING.store(true, Ordering::Relaxed);
    for _ in 0..1200 {
        let analysis = analyzer.analyze(&block);
        tracker.push(analysis.novelty);
        std::hint::black_box((analysis.frame, tracker.estimate()));
    }
    COUNTING.store(false, Ordering::Relaxed);

    let count = ALLOCATIONS.load(Ordering::Relaxed);
    assert_eq!(
        count, 0,
        "{count} allocations in 1200 hops of what runs in the audio callback"
    );
}
