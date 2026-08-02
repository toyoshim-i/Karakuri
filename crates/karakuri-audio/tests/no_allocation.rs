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

use karakuri_audio::analysis::{Analyzer, BLOCK, HOP};
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

/// **Music, not a still image.** A click train at 128 bpm on a broadband bed:
/// content in every bin so no branch inside the transform is skipped for being
/// lucky, *and a novelty curve that moves*, which is what actually gets the
/// estimator to run.
///
/// This is not a detail. Feeding one identical block over and over — which is
/// what this test used to do — leaves the spectral flux at zero, and an
/// estimator handed nothing but zeroes returns "no tempo" from its first few
/// lines and never reaches the autocorrelation, the fold, or the profile at
/// all. The test passed and covered none of `tempo`.
fn signal(hops: usize) -> Vec<f32> {
    let mut x: u32 = 0x9e37_7911;
    let period = (60.0 / 128.0 * 48_000.0) as usize;
    (0..BLOCK + hops * HOP)
        .map(|n| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            let noise = (x as f32 / u32::MAX as f32) * 2.0 - 1.0;
            let since = n % period;
            let click = if since < 64 {
                1.0 - since as f32 / 64.0
            } else {
                0.0
            };
            noise * 0.05 + click * 0.9
        })
        .collect()
}

#[test]
fn one_hop_of_callback_work_allocates_nothing() {
    let mut analyzer = Analyzer::new(48_000);
    let mut tracker = Tracker::new(analyzer.hop_seconds(), analyzer.window_lag(), 120.0);
    // Enough hops to cover every path a callback takes: the window filling, a
    // re-measurement (one push in every `ESTIMATE_INTERVAL_SECONDS`), and the
    // extrapolations between them.
    let hops = 1200;
    let signal = signal(hops);

    COUNTING.store(true, Ordering::Relaxed);
    for hop in 0..hops {
        let analysis = analyzer.analyze(&signal[hop * HOP..hop * HOP + BLOCK]);
        // The window centre moves under a live grid, and moving it is part of
        // what the callback does.
        tracker.set_centre_bpm(120.0 + (hop % 16) as f32);
        tracker.push(analysis.novelty);
        std::hint::black_box((analysis.frame, tracker.estimate()));
    }
    COUNTING.store(false, Ordering::Relaxed);

    // The estimator has to have actually run, or this counts the allocations of
    // a function that returned early.
    assert!(
        tracker.estimate().confidence > 0.0,
        "the estimator never produced an estimate, so nothing in `tempo` was measured"
    );

    let count = ALLOCATIONS.load(Ordering::Relaxed);
    assert_eq!(
        count, 0,
        "{count} allocations in 1200 hops of what runs in the audio callback"
    );
}
