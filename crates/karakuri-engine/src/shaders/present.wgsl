// The one place sRGB encoding happens.
//
// Everything upstream is linear and HDR. This pass samples the Rgba16Float
// target and writes it to a surface whose format carries the sRGB transfer
// function, so the hardware does the encoding on write and it happens exactly
// once. Tone mapping and bloom land in a later slice; for now values above 1.0
// clip, which is honest rather than correct.

@group(0) @binding(0) var hdr: texture_2d<f32>;
@group(0) @binding(1) var hdr_sampler: sampler;

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

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(hdr, hdr_sampler, in.uv).rgb;
    return vec4<f32>(c, 1.0);
}
