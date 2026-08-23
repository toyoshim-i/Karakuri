// The level meter: mean and peak luminance over one slot's linear HDR target.
//
// Two passes over one bind group. `reduce_tiles` folds the whole image into
// `{{WG}}` partial (sum, peak, unlit) triples, `reduce_total` folds those into
// `Level` the host copies out. They share a bind group so that the second pass
// can ask the texture how many texels there were, which is what turns a sum
// into a mean without the host having to tell either pass the resolution.
//
// Both are dispatched into the same compute pass, in this order. WebGPU
// executes dispatches within a pass in order and each sees the writes of the
// ones before it, which is the same guarantee `shaders/scan.wgsl` chains its
// scan levels on.
//
// **Rec.709 linear, and stated rather than assumed.** `docs/invariants.md`'s Color
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

// Where `peak` starts, so that it begins below anything a target can hold. Not
// 0.0: a generated L4 may write a negative colour — nothing in the pipeline
// rejects one — and a peak floored at zero would report that frame as having a
// peak it does not have. It is also the sentinel `reduce_total` tests for, in
// the one case where no texel was light at all.
const LOWEST: f32 = -3.4028235e38;

// Whether a luminance is a number the meter can add up — decided **on the
// bits** rather than by comparison.
//
// The obvious spelling is `l >= LOWEST && l <= HIGHEST`, and it works here, but
// WGSL explicitly permits an implementation to assume NaNs and infinities are
// absent and Metal's fast-math default is free to fold such a comparison to
// `true`. That would put a NaN back into the sum on the backend this ships on,
// silently, which is the whole defect this function exists to prevent. An
// exponent field of all ones is the encoding a NaN and an infinity share, and
// testing it is integer arithmetic no float optimisation may touch.
fn is_light(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7f800000u) != 0x7f800000u;
}

struct Level {
    mean: f32,
    peak: f32,
    // Texels whose luminance was not a finite number, this frame. An `f32`
    // because the whole reduction is one, and exact: counts stay integral in
    // `f32` up to 2^24, and a 4K frame is 8.3 million texels.
    bad: f32,
    // Not required by anything: a 12-byte staging buffer maps legally, since
    // `map_async` wants an eight-byte *start* and a four-byte end. It is here
    // so that this record and an entry of `partials` are the same 16 bytes,
    // which is one size to get right rather than two.
    _pad: f32,
};

@group(0) @binding(0) var source: texture_2d<f32>;
// (sum, peak, bad) per first-pass workgroup. Every entry is written every
// frame, so nothing here needs clearing between frames.
@group(0) @binding(1) var<storage, read_write> partials: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read_write> level: Level;

var<workgroup> sums: array<f32, {{WG}}>;
var<workgroup> peaks: array<f32, {{WG}}>;
var<workgroup> bads: array<f32, {{WG}}>;

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Fold `sums`, `peaks` and `bads` down to entry 0. Every barrier is outside
// the `lid < stride` branch and the trip count depends only on `WG`, so this runs
// in uniform control flow whoever calls it.
fn fold(lid: u32) {
    for (var stride = WG / 2u; stride > 0u; stride = stride / 2u) {
        workgroupBarrier();
        if lid < stride {
            sums[lid] = sums[lid] + sums[lid + stride];
            peaks[lid] = max(peaks[lid], peaks[lid + stride]);
            bads[lid] = bads[lid] + bads[lid + stride];
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
    var bad = 0.0;
    for (var k = 0u; k < iters; k = k + 1u) {
        let i = k * threads + wid.x * WG + lid;
        if i < total {
            let at = vec2<i32>(i32(i % dims.x), i32(i / dims.x));
            let l = luminance(textureLoad(source, at, 0i).rgb);
            // A texel that is not a finite luminance is counted and then left
            // out of both figures. One NaN admitted to `sum` makes the mean
            // NaN and takes the whole slot's reading with it, which is a
            // stray sprite costing an operator the number they set faders by.
            if is_light(l) {
                sum = sum + l;
                peak = max(peak, l);
            } else {
                bad = bad + 1.0;
            }
        }
    }

    sums[lid] = sum;
    peaks[lid] = peak;
    bads[lid] = bad;
    fold(lid);
    if lid == 0u {
        partials[wid.x] = vec3<f32>(sums[0], peaks[0], bads[0]);
    }
}

@compute @workgroup_size({{WG}})
fn reduce_total(@builtin(local_invocation_index) lid: u32) {
    sums[lid] = partials[lid].x;
    peaks[lid] = partials[lid].y;
    bads[lid] = partials[lid].z;
    fold(lid);
    if lid == 0u {
        let dims = textureDimensions(source);
        // Over the whole frame, black included. Most of the frame is black by
        // design and that is part of how much light there is — see `meter.rs`.
        // A texel that was not light divides in as zero, which is the same
        // thing black does and is the only reading that keeps a mean
        // comparable between a slot with a bad texel and one without.
        level.mean = sums[0] / f32(dims.x * dims.y);
        // **A frame with no light at all has no peak**, and `peaks[0]` is
        // still the sentinel it started at. Reporting that would hand a host
        // `-3.4e38` to format — 41 characters in a status line whose columns
        // are read at a glance in the dark — so it comes back as 0.0, which is
        // what the mean says about the same frame. `bad` is what distinguishes
        // it from black, and it says so in the only terms that are not a
        // judgement: every texel of it.
        level.peak = select(peaks[0], 0.0, peaks[0] == LOWEST);
        level.bad = bads[0];
    }
}
