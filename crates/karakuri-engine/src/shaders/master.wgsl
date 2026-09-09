// The master chain: three fixed passes between the mix's write and the tone
// map, in the order the console draws them — feedback, bloom, rgb shift.
//
// **One module and four fragment entry points**, over one bind group layout,
// because that is what makes "these are the chain's passes" checkable by
// reading one file rather than by auditing four. A pass reads `src`, may read
// `aux` (the retained frame for feedback, the bright buffer for bloom's second
// half), and writes one target. Which texture is bound where is `master.rs`'s
// routing; nothing here knows the order it is run in.
//
// **Linear HDR in, linear HDR out.** Everything here runs before the one tone
// map and before the one sRGB encode, on values that are expected to exceed
// 1.0 — see `docs/principles/0064-…`. Nothing clamps.
//
// **Colour only.** Alpha is coverage, and none of these three changes what a
// frame covers, so every pass writes the source's alpha through unchanged.
// That is `Deck::set_out`'s rule one step upstream, for its reason.
//
// **Every length is a fraction of the frame's height**, converted per axis
// from `textureDimensions`, never a count of texels. A radius in texels would
// make the render size part of the picture, and one frame is rendered at the
// largest enabled output's size and scaled into the rest
// (ADR-0247, `docs/principles/0086-…`), so the same session on a second
// display would bloom a different distance.

struct Chain {
    // Each is the amount its pass runs at, and `master.rs` records no pass at
    // all for an amount of zero — so a zero here is a shader that is not
    // reached rather than a multiply by nothing.
    feedback: f32,
    bloom: f32,
    shift: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> chain: Chain;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var aux: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

// Bloom's radius, in fractions of the frame's height: 1.2%, which is 8.6
// texels at 720 and 17.3 at 1440 — the same picture at two resolutions.
//
// **Fixed, and it is the one parameter of this chain that is deliberately not
// a control.** The tap count is what a radius costs, and a cost is known
// before it is paid (`docs/principles/0091-…`); a radius a hand could turn
// would either change the number of fetches per pixel at runtime or spread
// nine taps so far apart that the blur bands. See `master.rs`.
const BLOOM_RADIUS: f32 = 0.012;

// Nine taps per axis at `BLOOM_RADIUS / 4` apart, Gaussian with sigma half the
// radius: `exp(-i*i/8)`, normalised. Written out rather than computed so the
// weights are the same numbers on every backend.
const W0: f32 = 0.2041631;
const W1: f32 = 0.1801737;
const W2: f32 = 0.1238322;
const W3: f32 = 0.0662825;
const W4: f32 = 0.0276303;

// The most the three channels are pulled apart at `shift` of 1.0, in fractions
// of the frame's height: 2%, which is 14.4 texels either way at 720. Past that
// the three channels stop reading as one picture and start reading as three.
const SHIFT_MAX: f32 = 0.02;

// What is above the pipeline's honest white, and therefore what blooms.
//
// **1.0 and not a control**, because in a linear HDR pipeline 1.0 is not an
// arbitrary level: it is the top of the range the sRGB encode is honest about,
// and everything above it is light the display cannot show. What decides how
// much of a frame is up there is the level the frame enters this chain at —
// the Master bay's `out`, one pass upstream — which is exactly ADR-0224's
// point that the level before an effect is part of the picture.
const BLOOM_KNEE: f32 = 1.0;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// `present.wgsl`'s triangle, term for term: one oversized triangle rather than
// two, no seam and no vertex buffer.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

// One step of `r` frame-heights, in uv, per axis — isotropic in texels at any
// aspect ratio.
fn step_uv(r: f32) -> vec2<f32> {
    let dim = vec2<f32>(textureDimensions(src, 0));
    return vec2<f32>(r * dim.y / dim.x, r);
}

// **A retained frame that has gone bad is dropped rather than carried.**
//
// This is not `composite.wgsl`'s refusal to sanitise, and the difference is
// what is on the other side of the guard. There, a NaN belongs to a slot, and
// the answer is to skip the slot, because a fader pulled to zero is how an
// operator escapes broken material and it has to work on exactly the material
// that is broken (ADR-0042). Here the value belongs to **this pass's own
// history**: nothing renders it, no fader is under it, and a NaN read back
// into the frame it came from is written back into the history on the same
// frame — so it outlives its cause and there is nothing to skip.
//
// Exact for every value that is not one: `v == v` is false only for a NaN, and
// the magnitude test only for an infinity, so no finite texel is touched.
fn keepable(v: vec3<f32>) -> vec3<f32> {
    return select(vec3<f32>(0.0), v, (v == v) & (abs(v) < vec3<f32>(3.4e38)));
}

// **Feedback**: the retained cut of the previous frame, added to this one.
//
// `textureLoad` rather than a sample: the retained frame is the same size as
// this one and lines up texel for texel, so there is nothing to interpolate
// and a filtered read would only cost precision. Which cut `aux` holds — the
// frame as the mix wrote it, or this chain's exit — is chosen on the CPU and
// is invisible here (`master.rs`).
@fragment
fn fs_feedback(in: VsOut) -> @location(0) vec4<f32> {
    let at = vec2<i32>(in.clip.xy);
    let s = textureLoad(src, at, 0);
    let h = keepable(textureLoad(aux, at, 0).rgb);
    return vec4<f32>(s.rgb + chain.feedback * h, s.a);
}

// What one tap of the bright pass contributes: the light above the knee at
// that point, and nothing below it.
fn bright(uv: vec2<f32>) -> vec3<f32> {
    return max(textureSampleLevel(src, samp, uv, 0.0).rgb - vec3<f32>(BLOOM_KNEE), vec3<f32>(0.0));
}

// **Bloom, first half**: what is above the knee, blurred horizontally.
//
// Thresholded per tap, which is the same picture as thresholding the whole
// frame and then blurring it, and saves a pass to hold the difference in.
@fragment
fn fs_bloom_bright(in: VsOut) -> @location(0) vec4<f32> {
    let d = vec2<f32>(step_uv(BLOOM_RADIUS).x * 0.25, 0.0);
    var sum = vec3<f32>(0.0);
    sum += W4 * bright(in.uv - 4.0 * d);
    sum += W3 * bright(in.uv - 3.0 * d);
    sum += W2 * bright(in.uv - 2.0 * d);
    sum += W1 * bright(in.uv - d);
    sum += W0 * bright(in.uv);
    sum += W1 * bright(in.uv + d);
    sum += W2 * bright(in.uv + 2.0 * d);
    sum += W3 * bright(in.uv + 3.0 * d);
    sum += W4 * bright(in.uv + 4.0 * d);
    return vec4<f32>(sum, 1.0);
}

// **Bloom, second half**: the same blur down the other axis, scaled by the
// amount and added to the frame it came from. `src` is the frame, `aux` is
// what `fs_bloom_bright` wrote.
@fragment
fn fs_bloom_blend(in: VsOut) -> @location(0) vec4<f32> {
    let d = vec2<f32>(0.0, BLOOM_RADIUS * 0.25);
    var sum = vec3<f32>(0.0);
    sum += W4 * textureSampleLevel(aux, samp, in.uv - 4.0 * d, 0.0).rgb;
    sum += W3 * textureSampleLevel(aux, samp, in.uv - 3.0 * d, 0.0).rgb;
    sum += W2 * textureSampleLevel(aux, samp, in.uv - 2.0 * d, 0.0).rgb;
    sum += W1 * textureSampleLevel(aux, samp, in.uv - d, 0.0).rgb;
    sum += W0 * textureSampleLevel(aux, samp, in.uv, 0.0).rgb;
    sum += W1 * textureSampleLevel(aux, samp, in.uv + d, 0.0).rgb;
    sum += W2 * textureSampleLevel(aux, samp, in.uv + 2.0 * d, 0.0).rgb;
    sum += W3 * textureSampleLevel(aux, samp, in.uv + 3.0 * d, 0.0).rgb;
    sum += W4 * textureSampleLevel(aux, samp, in.uv + 4.0 * d, 0.0).rgb;
    let s = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    return vec4<f32>(s.rgb + chain.bloom * sum, s.a);
}

// **RGB shift**: red and blue sampled apart along x, green where it was.
//
// The centre tap is a `textureLoad` for the same reason `fs_feedback`'s is:
// at zero displacement the two outer taps land back on it, and a frame that
// went through this pass at an amount just above zero should differ from one
// that did not by what the shift is, not by what a filter did.
@fragment
fn fs_rgb_shift(in: VsOut) -> @location(0) vec4<f32> {
    let d = vec2<f32>(step_uv(SHIFT_MAX * chain.shift).x, 0.0);
    let centre = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    let r = textureSampleLevel(src, samp, in.uv + d, 0.0).r;
    let b = textureSampleLevel(src, samp, in.uv - d, 0.0).b;
    return vec4<f32>(r, centre.g, b, centre.a);
}
