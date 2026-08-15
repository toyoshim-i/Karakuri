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

    // **The divide is what makes the weight's scale arbitrary — everywhere
    // except in this guard.** Whatever constant every `w` was multiplied by
    // cancels in `acc.rgb / acc.a`, which is why the generated shader keeps its
    // weights inside `(0, 1]` instead of carrying the published `3e3` factor
    // into an `Rgba16Float` target holding unbounded HDR colour. The floor is
    // the one term the cancellation does not reach: it is compared against
    // `acc.a` directly, so a floor chosen for one weight scale is a different
    // floor under another.
    //
    // **So it is tied to the storage format instead**, and 2^-24 is where
    // `f16` stops representing anything at all. Anything the accumulation
    // target actually holds is either zero or at least this, so the guard can
    // only bite on a texel whose weight sum underflowed — where `acc.rgb`
    // underflowed with it, since `rgb <= max(c) * acc.a` term by term, and the
    // quotient stays bounded by the colour rather than exploding.
    //
    // **A floor above that is an invisible darkening of thin material**, and
    // this was 1e-5 for one commit — inherited from a weight function whose
    // `3e3` factor had been dropped. `a * w` goes as `a^2`, so a floor on it
    // eats alpha as its square root: 1e-5 started taking colour away at an
    // opacity of about 0.0034 and had removed 90% of it by 0.001. See
    // `a_lone_sprite_resolves_to_what_additive_accumulates_at_every_opacity`.
    //
    // A texel nothing drew on has `acc.a` of exactly zero — and `coverage` of
    // exactly zero with it, so the result is black either way; what the guard
    // buys there is that the intermediate is a number rather than a NaN,
    // because `0 * NaN` is NaN and would reach the mix.
    let colour = acc.rgb / max(acc.a, 5.96e-8);
    return vec4<f32>(colour * coverage, coverage);
}
