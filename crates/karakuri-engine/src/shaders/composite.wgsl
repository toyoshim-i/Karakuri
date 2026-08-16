// The L5 mix: up to four per-slot HDR targets folded into one HDR target, in
// slot order, each through its own blend mode.
//
// Linear in, linear out. No tone mapping and no sRGB anywhere near this — the
// pipeline is linear and HDR end to end and the one encode happens in the
// present pass, downstream of here.
//
// Three things about this shader are load-bearing rather than incidental:
//
//   - `textureLoad`, not `textureSample`. Every slot target is exactly the
//     size of the mix target, so the wanted texel is the one under the
//     fragment and there is nothing to filter. A sampler would introduce a
//     filter whose result at a texel centre is *supposed* to be the texel
//     itself; `textureLoad` makes it be the texel, which is what lets a deck
//     of one slot at unity reproduce a bare Set bit for bit — in colour
//     unconditionally, and in alpha for material whose coverage is in range.
//     See `deck.rs`'s `Blend::Add` for the one channel that can disagree.
//
//   - The four terms are folded in slot order, unrolled and written out.
//     Floating-point addition is not associative, so "the same seeds produce
//     the same pixels" is a claim about the order the fold is taken in as much
//     as about the values. Slot index fixes that order here; nothing about
//     which build finished first, or which slot went live most recently, can
//     reach it. With `over` in the vocabulary the order is no longer merely a
//     rounding question — a layer that covers is a layer that hides, and which
//     one hides which is the slot index and nothing else.
//
//   - **One formula, three blend functions.** Every mode is
//     `acc <- mix(acc, f(acc, gain * src), opacity)`, and the three cases below
//     are that expression already reduced by hand so that `add` costs exactly
//     what it cost when it was the only mode. That reduction is why `gain` and
//     `opacity` are separate numbers here at last: under `add` they collapse
//     into one multiply and always did, and under `over` and `max` they do not.
//     `gain` is the level the material arrives at, applied to colour alone;
//     `opacity` is the fader across the blend, and it is the only one of the
//     two that touches coverage.
//
// A slot that does not contribute is skipped rather than blended at zero.
// Skipping is exact for any contents whatsoever, including the ones an off-air
// slot's target happens to be holding from the last frame it rendered; `0.0 *
// x` is only zero for finite `x`, and an HDR target is allowed to hold an
// infinity or a NaN. That is why `live` is not just the residency: `deck.rs`
// clears it for a fader at silence too, so a slot's NaN cannot reach every
// channel of the mix. Which faders count as silence is a per-mode question and
// `deck.rs` answers it — an `over` layer at zero gain is a black card, which
// covers, so gain does not silence it and only `opacity` does.
//
// **Alpha is coverage**, `1 - prod(1 - a_i)` over the sprites that drew there,
// written by the L4 pass (see `node/renderer.rs` for the blend state that accumulates
// it) and composed here the same way whatever the colour mode is: coverage is
// "there is material at this texel", and that is an `over` question even when
// the colour is being added. So the one formula above holds for colour;
// **alpha is the exception and is always `over`.** Nothing downstream reads it
// today — the present pass tone maps `.rgb` — and it exists because `over`
// needs it and because M2's output routing hands a frame to something that
// will want to key on it.
//
// A layer this shader *skips* contributes no coverage either, which is the one
// place a level reaches alpha: `gain` scales colour and nothing else, but a
// gain of exactly zero silences the layer under `add` and `max`, and a
// silenced layer is not there at all. See `deck.rs`'s `Blend::silent_at`.
//
// **A mask is the fader varying across the frame**, and it multiplies opacity
// for exactly that reason: everything opacity already does — how much of the
// blend lands, and under `over` how much the layer covers — is what a mask
// wants done per texel. That is what makes a wipe fall out of the parts that
// were already here rather than needing a mode of its own: an incoming layer
// under `over`, with a linear mask whose position a transition is moving,
// hides the outgoing one exactly where the front has passed.

struct Mix {
    // Per slot: the level the material arrives at. Colour only.
    gain: vec4<f32>,
    // Per slot: the fader across the blend, `[0, 1]`.
    opacity: vec4<f32>,
    // Per slot: which `MODE_` below. See `deck.rs`'s `Blend`.
    mode: vec4<u32>,
    // Per slot: nonzero if the slot contributes to this frame's mix — Live,
    // and not at a fader its mode counts as silence. See above for why
    // silence is a skip.
    live: vec4<u32>,
    // Per slot: which `MASK_` below, and the three numbers that shape it.
    // See `deck.rs`'s `Mask`.
    mask: vec4<u32>,
    mask_angle: vec4<f32>,
    mask_position: vec4<f32>,
    mask_softness: vec4<f32>,
};

@group(0) @binding(0) var<uniform> mix_in: Mix;
@group(0) @binding(1) var slot0: texture_2d<f32>;
@group(0) @binding(2) var slot1: texture_2d<f32>;
@group(0) @binding(3) var slot2: texture_2d<f32>;
@group(0) @binding(4) var slot3: texture_2d<f32>;

const MODE_ADD: u32 = 0u;
const MODE_OVER: u32 = 1u;
const MODE_MAX: u32 = 2u;

const MASK_NONE: u32 = 0u;
const MASK_LINEAR: u32 = 1u;
const MASK_RADIAL: u32 = 2u;

// **How much of this layer reaches the mix at this texel**, in `[0, 1]`.
//
// A mask multiplies the layer's *opacity*, which is what makes it a mask
// rather than a second fader: `deck.rs` calls opacity the fader across the
// blend, and this is that fader varying across the frame. Under `over` it is
// therefore how much of the layer *covers* here too, which is what a wipe is —
// the incoming layer hides the outgoing one where the mask has arrived and
// nowhere else.
//
// `position` is how far the reveal has travelled, `[0, 1]`, and both ends are
// exact: **0 shows nothing anywhere and 1 shows everything everywhere**, for
// any softness. That is not decoration. A wipe is a transition on this number,
// so a `position` of 1 that left a corner half-lit would be a wipe that never
// finished, and `deck.rs` skips a layer at 0 outright — which is only sound if
// 0 really is nothing.
//
// `uv` is `[0, 1]` across the frame. `aspect` is width over height and is used
// by the radial mask alone, because an iris that is not round is not an iris,
// where a linear wipe's angle is measured in frame space on purpose: 45
// degrees should run corner to corner whatever shape the frame is.
fn mask_at(uv: vec2<f32>, aspect: f32, kind: u32, angle: f32, position: f32, softness: f32) -> f32 {
    if kind == MASK_NONE {
        return 1.0;
    }
    let centred = uv - vec2<f32>(0.5, 0.5);
    var d: f32;
    if kind == MASK_RADIAL {
        // Normalised so the corner is at 1, which is what makes `position` of
        // 1 reveal the whole frame rather than the inscribed circle.
        let wide = centred * vec2<f32>(aspect, 1.0);
        d = length(wide) / length(vec2<f32>(0.5 * aspect, 0.5));
    } else {
        let dir = vec2<f32>(cos(angle), sin(angle));
        // The half-extent of the frame along `dir`, so `d` spans exactly
        // `[0, 1]` whichever way the wipe runs. A fixed divisor would make a
        // diagonal wipe finish early and a vertical one finish late.
        let extent = abs(dir.x) * 0.5 + abs(dir.y) * 0.5;
        d = dot(centred, dir) / (2.0 * extent) + 0.5;
    }
    // The soft edge is *added* to the travel rather than eaten out of it, so
    // that the front starts entirely off one side and finishes entirely off
    // the other. Without that, a soft wipe would begin with a band already
    // showing and end with one still hidden.
    let soft = max(softness, 1.0 / 4096.0);
    let front = position * (1.0 + soft) - soft * 0.5;
    return clamp((front - d) / soft + 0.5, 0.0, 1.0);
}

// One oversized triangle, for the same reasons as `present.wgsl`: no seam, no
// vertex buffer, one fewer vertex.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

// One layer onto the accumulator.
//
// `src` is what the slot's L4 pass left: colour already premultiplied by the
// coverage of the sprites that wrote it — the pass adds `a * rgb` — and `a`
// the coverage itself. Premultiplied is what makes `over` one multiply-add
// rather than a divide by an alpha that is allowed to be zero, and it is what
// lets emissive material sum past what its coverage would permit, which is the
// whole reason `blend additive` is the L4 mode in the first place.
fn layer(acc: vec4<f32>, src: vec4<f32>, gain: f32, opacity: f32, mode: u32) -> vec4<f32> {
    // **Coverage, saturated on the way in.** Nothing bounds what an L4
    // fragment block assigns to alpha: the IR calls `color` linear RGB with
    // straight alpha and says values above 1.0 are expected, and a generated
    // procedure writing `color = vec4(rgb, 1.5)` compiles, costs and runs. The
    // L4 pass accumulates whatever that is, so a slot target's alpha is only a
    // coverage if the material kept it in range — and `1 - a` for `a = 1.5` is
    // negative light, while `a >= 2` makes `over` *amplify* what it was asked
    // to hide. Saturating here is the whole fix, and it belongs here rather
    // than in L4's output: clamping the fragment would change the colour too,
    // since additive blending multiplies colour by that same alpha.
    //
    // **Written as a comparison rather than as `clamp`, so that a NaN cannot
    // survive it on any backend.** WGSL defines `clamp` for floats as
    // `min(max(e, low), high)` and leaves `min`/`max` *indeterminate* when an
    // operand is NaN — Metal's suppress it, and a backend whose propagate would
    // put a NaN straight back into the mix. A comparison against NaN is false
    // everywhere, so the zero arm wins by the rule rather than by the vendor.
    //
    // `tests/deck.rs` asserts the outcome — no NaN in the mix — for a NaN
    // alpha, and it passes with `clamp` here too, because Metal's suppresses.
    // So this line is an argument rather than a measurement on this machine,
    // and it is written down as one: the test will catch it on the backend
    // where it matters, and there is no backend here on which to watch it.
    let covered = opacity * select(0.0, min(src.a, 1.0), src.a > 0.0);
    var rgb: vec3<f32>;
    switch mode {
        // `mix(acc, s + acc*(1 - s.a), o)` reduced, which is why `opacity`
        // reaches the coverage term and `gain` does not: turning a layer's
        // level down dims what it draws, and a black card still covers what is
        // behind it. Turning its *fader* down is what stops it covering.
        case MODE_OVER: {
            rgb = (gain * opacity) * src.rgb + acc.rgb * (1.0 - covered);
        }
        // `mix(acc, max(acc, g*s), o)` — a crossfade *into* the maximum, so
        // the fader still runs the layer smoothly in and out rather than
        // switching it. Scale-free, which is why it is here and `screen` is
        // not: `screen` is `d + s - d*s` and means nothing above 1.0, and
        // nothing has tone mapped yet at this point in the pipeline.
        case MODE_MAX: {
            rgb = mix(acc.rgb, max(acc.rgb, gain * src.rgb), opacity);
        }
        // MODE_ADD — `mix(acc, acc + g*s, o)` reduced, and identical term for
        // term and rounding for rounding to the single `weight * src` this
        // shader did when `add` was the only mode there was. It is also the
        // `default` arm, so anything a newer build wrote that this one does
        // not know arrives as the mode this deck comes up in, which is the one
        // that cannot look broken.
        default: {
            rgb = acc.rgb + (gain * opacity) * src.rgb;
        }
    }
    return vec4<f32>(rgb, covered + acc.a * (1.0 - covered));
}

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    // `pos` is the framebuffer coordinate at the pixel centre, so truncating
    // it is the texel index rather than a rounding decision.
    let at = vec2<i32>(pos.xy);
    // Every slot target is the size of the mix target — the assertion in
    // `Frame::render` is what makes that true — so one of them answers for the
    // frame. `uv` is at the texel centre, which is where the mask is sampled
    // for the same reason `textureLoad` is used rather than a sampler.
    let dims = vec2<f32>(textureDimensions(slot0));
    let uv = pos.xy / dims;
    let aspect = dims.x / dims.y;

    var acc = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if mix_in.live.x != 0u {
        let m = mask_at(uv, aspect, mix_in.mask.x, mix_in.mask_angle.x, mix_in.mask_position.x, mix_in.mask_softness.x);
        acc = layer(acc, textureLoad(slot0, at, 0i), mix_in.gain.x, mix_in.opacity.x * m, mix_in.mode.x);
    }
    if mix_in.live.y != 0u {
        let m = mask_at(uv, aspect, mix_in.mask.y, mix_in.mask_angle.y, mix_in.mask_position.y, mix_in.mask_softness.y);
        acc = layer(acc, textureLoad(slot1, at, 0i), mix_in.gain.y, mix_in.opacity.y * m, mix_in.mode.y);
    }
    if mix_in.live.z != 0u {
        let m = mask_at(uv, aspect, mix_in.mask.z, mix_in.mask_angle.z, mix_in.mask_position.z, mix_in.mask_softness.z);
        acc = layer(acc, textureLoad(slot2, at, 0i), mix_in.gain.z, mix_in.opacity.z * m, mix_in.mode.z);
    }
    if mix_in.live.w != 0u {
        let m = mask_at(uv, aspect, mix_in.mask.w, mix_in.mask_angle.w, mix_in.mask_position.w, mix_in.mask_softness.w);
        acc = layer(acc, textureLoad(slot3, at, 0i), mix_in.gain.w, mix_in.opacity.w * m, mix_in.mode.w);
    }
    return acc;
}
