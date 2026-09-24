use super::types::{Arrangement, Solved};
use crate::{Axis, Sizing};

/// Computes the usable extent for node `i` bottom-up along `parent` axis, writing to `s.usable[i]`.
///
/// Fixed leaves report their size; flex leaves are unbounded. Splits sum child extents
/// along matching axes, or report unbounded when orthogonal.
pub(crate) fn measure(a: &Arrangement, s: &mut Solved, i: usize, parent: Option<Axis>) -> f32 {
    let content = match a.split_of(i) {
        None => match a.nodes[i].sizing {
            Sizing::Fixed(size) => size.max(0.0),
            Sizing::Flex(_) => f32::INFINITY,
        },
        Some((axis, divider)) => {
            let mut sum = 0.0;
            let mut tiled = 0usize;
            for k in 0..a.child_count(i) {
                let c = a.child(i, k);
                let child = measure(a, s, c, Some(axis));
                // Closed children contribute zero extent, but their divider is
                // counted in tiled gap calculations.
                if !a.out_of_layout(c) {
                    sum += child;
                }
                if a.placed(c) {
                    tiled += 1;
                }
            }
            let gaps = tiled.saturating_sub(1) as f32;
            if tiled == 0 {
                0.0
            } else if parent == Some(axis) {
                sum + divider.max(0.0) * gaps
            } else {
                f32::INFINITY
            }
        }
    };
    // `min` before `max` so that a NaN in either bound leaves the other one
    // holding, and so nothing negative ever reaches a claim.
    let usable = content.min(a.nodes[i].max).max(0.0);
    s.usable[i] = usable;
    usable
}

/// Returns child `c`'s extent claim along the split axis, capped to its usable content extent.
pub(crate) fn claim(s: &Solved, c: usize, size: f32) -> f32 {
    size.max(0.0).min(s.usable[c])
}

/// Solves the subtree rooted at `i` against scratch buffer `s` without mutating arrangement `a` (P-0082).
pub(crate) fn solve_subtree(a: &Arrangement, s: &mut Solved, i: usize) {
    if a.split_of(i).is_none() {
        return;
    }
    solve_split(a, s, i);
    // The scratch is finished with by now, which is what lets one buffer
    // serve the whole recursion.
    for k in 0..a.child_count(i) {
        let c = a.child(i, k);
        solve_subtree(a, s, c);
    }
}

pub(crate) fn solve_split(a: &Arrangement, s: &mut Solved, split: usize) {
    let Some((axis, _)) = a.split_of(split) else {
        return;
    };
    let rect = s.rects[split];
    let extent = axis.extent(rect).max(0.0);
    let n = a.child_count(split);
    let divider = a.effective_divider(split, extent);
    let avail = a.avail(split, extent);

    for k in 0..n {
        let c = a.child(split, k);
        s.sizes[k] = 0.0;
        s.frozen[k] = a.out_of_layout(c);
    }

    // Step 4. One pass per child is enough, since every pass but the last
    // freezes one; the `+ 1` is the pass that settles and breaks.
    for _ in 0..=n {
        let mut frozen_sum = 0.0;
        let mut claimed = 0.0;
        let mut stored = 0.0;
        let mut weight = 0.0;
        for k in 0..n {
            if s.frozen[k] {
                frozen_sum += s.sizes[k];
                continue;
            }
            let c = a.child(split, k);
            match a.nodes[c].sizing {
                Sizing::Fixed(size) => {
                    claimed += claim(s, c, size);
                    stored += size.max(0.0);
                }
                Sizing::Flex(w) => weight += w.max(0.0),
            }
        }

        if weight > 0.0 {
            let pool = (avail - frozen_sum - claimed).max(0.0);
            for k in 0..n {
                if s.frozen[k] {
                    continue;
                }
                let c = a.child(split, k);
                let size = match a.nodes[c].sizing {
                    Sizing::Fixed(size) => claim(s, c, size),
                    Sizing::Flex(w) => pool * w.max(0.0) / weight,
                };
                s.sizes[k] = size;
            }
        } else {
            // Distribute discrepancy proportionally across fixed children when no flexible
            // children remain unfrozen, scaling from stored sizes up to maxima (ADR-0157).
            let pool = (avail - frozen_sum).max(0.0);
            let scale = if stored > 0.0 { pool / stored } else { 0.0 };
            for k in 0..n {
                if s.frozen[k] {
                    continue;
                }
                let c = a.child(split, k);
                let size = match a.nodes[c].sizing {
                    Sizing::Fixed(size) => size.max(0.0) * scale,
                    Sizing::Flex(_) => 0.0,
                };
                s.sizes[k] = size;
            }
        }

        let mut bounded = false;
        for k in 0..n {
            if s.frozen[k] {
                continue;
            }
            let c = a.child(split, k);
            // Declared minimum capped by usable content extent to avoid reserving empty space.
            let min = a.nodes[c].min.max(0.0).min(s.usable[c]);
            let max = a.nodes[c].max;
            if s.sizes[k] < min {
                s.sizes[k] = min;
                s.frozen[k] = true;
                bounded = true;
            } else if s.sizes[k] > max {
                s.sizes[k] = max;
                s.frozen[k] = true;
                bounded = true;
            }
        }
        if !bounded {
            break;
        }
    }

    // Step 5.
    let mut total = 0.0;
    for k in 0..n {
        total += s.sizes[k];
    }
    if total > avail {
        let scale = if total > 0.0 { avail / total } else { 0.0 };
        for k in 0..n {
            s.sizes[k] *= scale;
        }
    }

    let mut cursor = axis.origin(rect);
    let mut done = 0;
    let tiled = a.placed_count(split);
    for k in 0..n {
        let c = a.child(split, k);
        s.rects[c] = axis.slice(rect, cursor, s.sizes[k].max(0.0));
        cursor += s.sizes[k].max(0.0);
        // A closed child is placed at zero extent and **still gets its
        // divider**, which is the whole of the edge it keeps: the gap lands
        // between the window's own edge and whatever took the pane's width.
        if a.placed(c) {
            done += 1;
            if done < tiled {
                cursor += divider;
            }
        }
    }
}
