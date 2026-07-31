// The one place sRGB encoding happens, and the one place tone mapping happens.
//
// Everything upstream is linear and HDR. This pass samples the Rgba16Float
// target, tone maps it back into a displayable `[0, 1]`, and writes to a
// surface whose format carries the sRGB transfer function, so the hardware
// does the encoding on write and it happens exactly once — on values a tone
// mapper already brought into range, which is the only values sRGB encoding
// is honest about.
//
// The operator is a uniform (`tonemap.op`), not a shader variant: every
// branch below is always compiled in, and switching Clamp for ACES for AgX is
// a `queue.write_buffer`, never a pipeline rebuild.

@group(0) @binding(0) var hdr: texture_2d<f32>;
@group(0) @binding(1) var hdr_sampler: sampler;

struct Tonemap {
    op: u32,
    exposure: f32,
    // Reinhard's own control: the input level that maps to exactly 1.0.
    // Ignored by every other operator.
    white_point: f32,
    _pad: u32,
};
@group(0) @binding(2) var<uniform> tonemap: Tonemap;

const OP_CLAMP: u32 = 0u;
const OP_REINHARD: u32 = 1u;
const OP_ACES: u32 = 2u;
const OP_AGX: u32 = 3u;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// One oversized triangle rather than two: no seam down the diagonal, one fewer
// vertex, and no vertex buffer.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

// Extended Reinhard (Reinhard & Devlin's white-point variant, not the
// textbook `c / (1 + c)`): a chosen input level maps to exactly 1.0 instead
// of every level asymptotically approaching it, so a bright core clips at a
// controllable point rather than forever dimming toward white without ever
// reaching it. Per channel — which is exactly what lets it shift hue as it
// compresses: a saturated colour whose one channel overflows desaturates
// toward the channels that have not overflowed yet, reading as the material
// "losing colour" under load.
fn reinhard_extended(c: vec3<f32>, white_point: f32) -> vec3<f32> {
    let w2 = white_point * white_point;
    return (c * (1.0 + c / w2)) / (1.0 + c);
}

// Narkowicz's fitted approximation to the ACES reference rendering transform
// (Krzysztof Narkowicz, "ACES Filmic Tone Mapping Curve", 2016) — the
// industry-default look, because most footage anyone has watched went through
// something like it. Per channel, same as `reinhard_extended` above, so the
// same hue-shifting consequence follows: a blown-out blue core reads as
// turning cyan and then white rather than staying blue and getting brighter.
fn aces(c: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let cc = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((c * (a * c + b)) / (c * (cc * c + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

// The default-contrast sigmoid approximation used by the minimal AgX ports
// (a degree-6 fit to Blender/Troy Sobotka's AgX contrast curve, in the form
// widely circulated as "AgX Minimal", e.g. in Bevy's and Godot's shader
// ports). Operates on the log2-encoded, already-[0,1]-normalised value from
// `agx` below.
fn agx_contrast_approx(x: vec3<f32>) -> vec3<f32> {
    let x2 = x * x;
    let x4 = x2 * x2;
    return 15.5 * x4 * x2 - 40.14 * x4 * x + 31.96 * x4 - 6.868 * x2 * x + 0.4298 * x2 + 0.1191 * x - 0.00232;
}

// AgX, approximated. Unlike `reinhard_extended` and `aces` above, this does
// not compress each channel independently: it rotates into a working space
// first (`agx_mat`, an inset matrix that pulls the primaries in slightly),
// compresses log-encoded values with one shared curve, then rotates back
// (`agx_eotf`). Sharing the curve across channels is what keeps an
// overdriven channel from running away from its neighbours before they catch
// up — the property this project's material wants, because additive,
// saturated point sprites push one channel far past 1.0 unevenly, and
// per-channel compression is exactly what reads as the material losing its
// colour when it gets loud.
fn agx(c: vec3<f32>) -> vec3<f32> {
    let agx_mat = mat3x3<f32>(
        vec3<f32>(0.842479062253094, 0.0784335999999992, 0.0792237451477643),
        vec3<f32>(0.0423282422610123, 0.878468636469772, 0.0791661274605434),
        vec3<f32>(0.0423756549057051, 0.0784336, 0.879142973793104),
    );
    let min_ev = -12.47393;
    let max_ev = 4.026069;

    // `max` before `log2`: a zero or negative input has no log, and additive
    // blending never produces negative light, so clamping to a tiny positive
    // epsilon here loses nothing real.
    var v = agx_mat * max(c, vec3<f32>(1e-10));
    v = clamp(log2(v), vec3<f32>(min_ev), vec3<f32>(max_ev));
    v = (v - min_ev) / (max_ev - min_ev);
    return agx_contrast_approx(v);
}

fn agx_eotf(c: vec3<f32>) -> vec3<f32> {
    let agx_mat_inv = mat3x3<f32>(
        vec3<f32>(1.19687900512017, -0.0980208811401368, -0.0990297440797205),
        vec3<f32>(-0.0528968517574562, 1.15190312990417, -0.0989611768448433),
        vec3<f32>(-0.0529716355144438, -0.0980434501171241, 1.15107367264116),
    );
    return agx_mat_inv * c;
}

// Applies the selected operator to one linear HDR colour, after exposure.
// Exposure lives in the same uniform as the operator because it is the
// operator's own control — how much light this transfer sees before it
// compresses — not a procedure's `param exposure` (how bright that material
// is) and not L5's future per-Set gain (how it balances against the others).
fn tonemap_apply(c_in: vec3<f32>) -> vec3<f32> {
    let c = c_in * tonemap.exposure;
    if (tonemap.op == OP_REINHARD) {
        return reinhard_extended(c, max(tonemap.white_point, 1e-4));
    } else if (tonemap.op == OP_ACES) {
        return aces(c);
    } else if (tonemap.op == OP_AGX) {
        return clamp(agx_eotf(agx(c)), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    // OP_CLAMP, and the fallback for anything else: `min(c, 1.0)`, the
    // pre-tone-mapper behaviour kept as the honest baseline.
    return min(c, vec3<f32>(1.0));
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(hdr, hdr_sampler, in.uv).rgb;
    return vec4<f32>(tonemap_apply(c), 1.0);
}
