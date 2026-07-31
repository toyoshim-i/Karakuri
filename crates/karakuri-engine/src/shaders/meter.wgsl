// The level meter: mean and peak luminance over one slot's linear HDR target.
//
// Two passes over one bind group. `reduce_tiles` folds the whole image into
// `{{WG}}` partial (sum, peak) pairs, `reduce_total` folds those into the one
// `Level` the host copies out. They share a bind group so that the second pass
// can ask the texture how many texels there were, which is what turns a sum
// into a mean without the host having to tell either pass the resolution.
//
// Both are dispatched into the same compute pass, in this order. WebGPU
// executes dispatches within a pass in order and each sees the writes of the
// ones before it, which is the same guarantee `shaders/scan.wgsl` chains its
// scan levels on.
//
// **Rec.709 linear, and stated rather than assumed.** `README.md`'s Color
// invariant is that the pipeline is linear and HDR end to end; the primaries
// that goes with are Rec.709, so luminance is 0.2126 R + 0.7152 G + 0.0722 B
// and not the average of the three. The difference is not cosmetic — green
// carries ten times the luminance blue does at equal magnitude, so a meter that
// averaged would report a saturated blue Set as being as bright as a green one
// and an operator matching faders by that number would be matching the wrong
// thing.

const WG: u32 = {{WG}}u;
// One workgroup of the second pass consumes exactly one partial per thread, so
// the first pass dispatches exactly `WG` workgroups. The two numbers are the
// same number for that reason; `meter.rs` templates both from one constant.
const TILES: u32 = {{WG}}u;

// Smaller than any finite f32, so `peak` starts below anything a target can
// hold. Not 0.0: a generated L4 may write a negative colour — nothing in the
// pipeline rejects one — and a peak floored at zero would report that frame as
// having a peak it does not have.
const LOWEST: f32 = -3.4028235e38;

struct Level {
    mean: f32,
    peak: f32,
};

@group(0) @binding(0) var source: texture_2d<f32>;
// (sum, peak) per first-pass workgroup. Every entry is written every frame, so
// nothing here needs clearing between frames.
@group(0) @binding(1) var<storage, read_write> partials: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> level: Level;

var<workgroup> sums: array<f32, {{WG}}>;
var<workgroup> peaks: array<f32, {{WG}}>;

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Fold `sums` and `peaks` down to entry 0. Every barrier is outside the
// `lid < stride` branch and the trip count depends only on `WG`, so this runs
// in uniform control flow whoever calls it.
fn fold(lid: u32) {
    for (var stride = WG / 2u; stride > 0u; stride = stride / 2u) {
        workgroupBarrier();
        if lid < stride {
            sums[lid] = sums[lid] + sums[lid + stride];
            peaks[lid] = max(peaks[lid], peaks[lid + stride]);
        }
    }
    workgroupBarrier();
}

@compute @workgroup_size({{WG}})
fn reduce_tiles(
    @builtin(local_invocation_index) lid: u32,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let dims = textureDimensions(source);
    let total = dims.x * dims.y;
    let threads = TILES * WG;

    // The strided walk is written with a uniform trip count and a guard rather
    // than as `for (i = start; i < total; i += threads)`, whose trip count
    // differs between invocations of the same workgroup. `fold` below contains
    // barriers, and keeping every loop bound uniform is the cheapest way to
    // keep that unarguable.
    let iters = (total + threads - 1u) / threads;
    var sum = 0.0;
    var peak = LOWEST;
    for (var k = 0u; k < iters; k = k + 1u) {
        let i = k * threads + wid.x * WG + lid;
        if i < total {
            let at = vec2<i32>(i32(i % dims.x), i32(i / dims.x));
            let l = luminance(textureLoad(source, at, 0i).rgb);
            sum = sum + l;
            peak = max(peak, l);
        }
    }

    sums[lid] = sum;
    peaks[lid] = peak;
    fold(lid);
    if lid == 0u {
        partials[wid.x] = vec2<f32>(sums[0], peaks[0]);
    }
}

@compute @workgroup_size({{WG}})
fn reduce_total(@builtin(local_invocation_index) lid: u32) {
    sums[lid] = partials[lid].x;
    peaks[lid] = partials[lid].y;
    fold(lid);
    if lid == 0u {
        let dims = textureDimensions(source);
        // Over the whole frame, black included. Most of the frame is black by
        // design and that is part of how much light there is — see `meter.rs`.
        level.mean = sums[0] / f32(dims.x * dims.y);
        level.peak = peaks[0];
    }
}
