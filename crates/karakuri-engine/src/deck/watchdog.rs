//! Runtime performance watchdog evaluation and overload gating (ADR-0269, ADR-0316).

use std::time::Duration;
use web_time::Instant;

use super::{Deck, DeckSlot};
use crate::Residency;

impl Deck {
    /// Returns true if the slot has been stopped by the performance watchdog.
    pub fn overloaded(&self, slot: DeckSlot) -> bool {
        self.slots[slot.index()].swap.overloaded()
    }

    /// Sets whether the specified slot is stopped due to budget overload gating.
    pub fn set_overloaded(&mut self, slot: DeckSlot, overloaded: bool) {
        if let Some(s) = self.slots.get_mut(slot.index()) {
            s.swap.set_overloaded(overloaded);
        }
    }

    /// Grants a temporary grace period during which watchdog gating is suppressed.
    ///
    /// Use this during expected transient hitches (e.g. opening, closing, or resizing
    /// auxiliary projector windows / surfaces) to prevent false-positive overload gating.
    pub fn grant_watchdog_grace(&mut self, duration: Duration) {
        let until = Instant::now() + duration;
        self.watchdog_grace_until = Some(
            self.watchdog_grace_until
                .map_or(until, |prev| prev.max(until)),
        );
        self.overload_since = None;
        self.last_frame_dt_ms = None;
        self.last_frame_at = Some(Instant::now());
        for slot in &mut self.slots {
            slot.swap.reset_period();
        }
    }

    /// Evaluates runtime performance watchdog against `watchdog_threshold_ms`.
    ///
    /// Automatically gates active live slots if the rolling median frame period
    /// continuously exceeds the overload threshold (e.g. dropping below 20 FPS)
    /// for at least `watchdog_sustained` (default 1.0 second), after an initial
    /// startup grace period (`watchdog_warmup`, default 3.0 seconds) and any
    /// granted transient grace periods.
    /// Halts updates to preserve console responsiveness.
    /// Returns the index of the gated slot if a gating decision occurred.
    pub fn watchdog_gate(&mut self) -> Option<usize> {
        let now = Instant::now();
        // Warmup grace period: ignore gating during initial startup
        if now.duration_since(self.started_at) < self.watchdog_warmup {
            self.overload_since = None;
            return None;
        }

        // Transient grace period: ignore gating during expected transient operations (e.g. window open/close)
        if self.watchdog_grace_until.is_some_and(|until| now < until) {
            self.overload_since = None;
            self.last_frame_dt_ms = None;
            self.last_frame_at = Some(now);
            for slot in &mut self.slots {
                slot.swap.reset_period();
            }
            return None;
        } else if self.watchdog_grace_until.is_some() {
            // Grace period just expired on this frame: clear grace state and reset counters
            // so post-grace evaluation begins from clean steady-state frames.
            self.watchdog_grace_until = None;
            self.overload_since = None;
            self.last_frame_dt_ms = None;
            self.last_frame_at = Some(now);
            for slot in &mut self.slots {
                slot.swap.reset_period();
            }
            return None;
        }

        let dt_ms = self
            .last_frame_at
            .map(|t| now.duration_since(t).as_secs_f32() * 1000.0);
        let period = self
            .last_frame_dt_ms
            .into_iter()
            .chain(dt_ms)
            .chain(self.frame_period_ms())
            .fold(0.0f32, f32::max);
        if period <= 0.0 {
            return None;
        }
        if period >= self.watchdog_threshold_ms {
            let frame_duration = Duration::from_secs_f32(period / 1000.0);
            let start = *self
                .overload_since
                .get_or_insert_with(|| now.checked_sub(frame_duration).unwrap_or(now));
            if now.duration_since(start) >= self.watchdog_sustained {
                let to_gate = self
                    .activation_order
                    .iter()
                    .rev()
                    .copied()
                    .find(|&slot_idx| {
                        if let Some(s) = self.slots.get(slot_idx) {
                            let in_mix = s.online && s.opacity > 0.0;
                            s.effective == Residency::Live && !s.swap.overloaded() && in_mix
                        } else {
                            false
                        }
                    })
                    .or_else(|| {
                        self.slots.iter().enumerate().rev().find_map(|(i, s)| {
                            (s.effective == Residency::Live && !s.swap.overloaded()).then_some(i)
                        })
                    });

                if let Some(slot_idx) = to_gate {
                    eprintln!(
                        "Watchdog overload gating slot {slot_idx}: period={period:.1}ms (last_dt={:?}ms, dt={:?}ms, rolling_median={:?}ms) >= threshold={:.1}ms, sustained for {:?}",
                        self.last_frame_dt_ms,
                        dt_ms,
                        self.frame_period_ms(),
                        self.watchdog_threshold_ms,
                        now.duration_since(start)
                    );
                    self.slots[slot_idx].swap.set_overloaded(true);
                    // Reset overload timer so subsequent gates require another sustained duration
                    self.overload_since = None;
                    return Some(slot_idx);
                }
            }
        } else {
            self.overload_since = None;
        }
        None
    }

    /// Sets the frame period threshold in milliseconds for watchdog overload gating.
    pub fn set_watchdog_threshold_ms(&mut self, threshold_ms: f32) {
        if threshold_ms.is_finite() && threshold_ms > 0.0 {
            self.watchdog_threshold_ms = threshold_ms;
        }
    }

    /// Returns the frame period threshold in milliseconds for watchdog overload gating.
    pub fn watchdog_threshold_ms(&self) -> f32 {
        self.watchdog_threshold_ms
    }

    /// Configures the warmup grace period and sustained period overrun duration for watchdog gating.
    pub fn set_watchdog_timing(&mut self, warmup: Duration, sustained: Duration) {
        self.watchdog_warmup = warmup;
        self.watchdog_sustained = sustained;
    }
}
