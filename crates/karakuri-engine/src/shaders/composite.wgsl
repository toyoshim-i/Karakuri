// The L5 mix: up to four per-slot HDR targets summed into one HDR target.
//
// Linear in, linear out. No tone mapping and no sRGB anywhere near this — the
// pipeline is linear and HDR end to end and the one encode happens in the
// present pass, downstream of here.
//
// Two things about this shader are load-bearing rather than incidental:
//
//   - `textureLoad`, not `textureSample`. Every slot target is exactly the
//     size of the mix target, so the wanted texel is the one under the
//     fragment and there is nothing to filter. A sampler would introduce a
//     filter whose result at a texel centre is *supposed* to be the texel
//     itself; `textureLoad` makes it be the texel, which is what lets a deck
//     of one slot at weight 1.0 reproduce a bare Set bit for bit.
//
//   - The four terms are summed in slot order, unrolled and written out.
//     Floating-point addition is not associative, so "the same seeds produce
//     the same pixels" is a claim about the order the sum is taken in as much
//     as about the values. Slot index fixes that order here; nothing about
//     which build finished first, or which slot went live most recently, can
//     reach it.
//
// A slot that does not contribute is skipped rather than multiplied by zero.
// Skipping is exact for any contents whatsoever, including the ones an off-air
// slot's target happens to be holding from the last frame it rendered; `0.0 *
// x` is only zero for finite `x`, and an HDR target is allowed to hold an
// infinity or a NaN. That is why `live` is not just the residency: `deck.rs`
// clears it for a weight of zero too, so a fader pulled to silence cannot put
// one slot's NaN into every channel of the mix.

struct Mix {
    // Per slot: gain * opacity. See `deck.rs` for why those are two numbers
    // that arrive here as one.
    weight: vec4<f32>,
    // Per slot: nonzero if the slot contributes to this frame's mix — Live,
    // and at a weight that is not zero. See below for why zero is a skip.
    live: vec4<u32>,
};

@group(0) @binding(0) var<uniform> mix: Mix;
@group(0) @binding(1) var slot0: texture_2d<f32>;
@group(0) @binding(2) var slot1: texture_2d<f32>;
@group(0) @binding(3) var slot2: texture_2d<f32>;
@group(0) @binding(4) var slot3: texture_2d<f32>;

// One oversized triangle, for the same reasons as `present.wgsl`: no seam, no
// vertex buffer, one fewer vertex.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    // `pos` is the framebuffer coordinate at the pixel centre, so truncating
    // it is the texel index rather than a rounding decision.
    let at = vec2<i32>(pos.xy);

    var acc = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if mix.live.x != 0u { acc = acc + mix.weight.x * textureLoad(slot0, at, 0i); }
    if mix.live.y != 0u { acc = acc + mix.weight.y * textureLoad(slot1, at, 0i); }
    if mix.live.z != 0u { acc = acc + mix.weight.z * textureLoad(slot2, at, 0i); }
    if mix.live.w != 0u { acc = acc + mix.weight.w * textureLoad(slot3, at, 0i); }
    return acc;
}
