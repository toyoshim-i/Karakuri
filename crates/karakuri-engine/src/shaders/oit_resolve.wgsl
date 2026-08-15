// The second half of `blend weighted`: two accumulations folded back into one
// slot target.
//
// The L4 pass leaves two things behind. `accum` holds `sum(c * a * w)` in rgb
// and `sum(a * w)` in alpha, where `w` is the depth weight the generated
// fragment stage computed. `reveal` holds `prod(1 - a)` — how much of whatever
// is behind this layer is still visible. Both are order-independent: a sum and
// a product do not care which fragment arrived first, which is the whole reason
// this exists rather than a depth sort. Compaction moves elements between
// steps, so any technique that depended on draw order would be at war with it.
//
// **What leaves here is exactly what the additive path leaves**, and that is
// not a coincidence to be grateful for — it is what keeps L5 out of this. A
// slot target means "colour premultiplied by coverage, and coverage in alpha";
// `composite.wgsl` reads it that way whatever produced it, so `blend weighted`
// reaches the mix without the mix knowing the mode exists.
//
// `textureLoad`, not `textureSample`, for `composite.wgsl`'s reason: both
// targets are exactly the size of the frame, so the wanted texel is the one
// under the fragment and there is nothing to filter.

@group(0) @binding(0) var accum: texture_2d<f32>;
@group(0) @binding(1) var reveal: texture_2d<f32>;

// One oversized triangle, for the same reasons as `present.wgsl`: no seam, no
// vertex buffer, one fewer vertex.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let at = vec2<i32>(pos.xy);
    let acc = textureLoad(accum, at, 0i);
    // `1 - prod(1 - a)`: the probability that something drew here, which is the
    // same number the additive path accumulates in its alpha channel by a
    // different route.
    let coverage = 1.0 - textureLoad(reveal, at, 0i).r;

    // **The divide is what makes the weight's scale arbitrary.** Whatever
    // constant every `w` was multiplied by cancels here, which is why the
    // generated shader keeps its weights inside `(0, 1]` instead of carrying
    // the published `3e3` factor into an `Rgba16Float` target holding unbounded
    // HDR colour.
    //
    // The guard is not a formality. A texel nothing drew on has `acc.a` of
    // exactly zero — and `coverage` of exactly zero with it, so the result is
    // black either way; the guard is there so the intermediate is a number
    // rather than a NaN, because `0 * NaN` is NaN and would reach the mix.
    let colour = acc.rgb / max(acc.a, 1e-5);
    return vec4<f32>(colour * coverage, coverage);
}
