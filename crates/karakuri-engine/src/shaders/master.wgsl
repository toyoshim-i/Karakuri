// The master chain's one remaining hand-written pass: the retention.
//
// **What is left of this file is the copy**, and what left it is the three
// passes. Feedback, bloom and rgb shift were four fragment entry points here
// until 2026-09-10; they are `examples/feedback.kir`, `examples/bloom.kir` and
// `examples/rgb_shift.kir` now, compiled through `karakuri_codegen::l5` —
// `docs/adr/0340-…` is the decision and `tests/master.rs` is what holds each
// replacement to the pass it replaced. `step_uv` went with them, into
// `frame_step`; `BLOOM_RADIUS`, `BLOOM_KNEE`, `SHIFT_MAX` and the five Gaussian
// weights went with them, into literals a reader of a `.kir` can see.
//
// **What could not go is `keepable`**, and it moved in the other direction:
// ADR-0340 sanitises the retained frame **where it is written** rather than
// where it is read, so that a `.kir` neither needs the guard nor can leave it
// out. That makes the retention a pass rather than a `copy_texture_to_texture`
// — which is the one thing in this chain that is still WGSL, and it is here.
//
// **Still exact for every value that is not one.** `v == v` is false only for a
// NaN and the magnitude test only for an infinity, so no finite texel is
// touched and a retained frame is bit for bit the frame it was copied from.
// A `textureLoad` and not a sample: the retention is the same size as its
// source and lines up texel for texel, so there is nothing to interpolate.

@group(0) @binding(0) var src: texture_2d<f32>;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
};

// `present.wgsl`'s triangle, term for term: one oversized triangle rather than
// two, no seam and no vertex buffer. No varying: the only coordinate this pass
// needs is its own fragment position.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

// **A retained frame that has gone bad is dropped rather than carried.**
//
// This is not `composite.wgsl`'s refusal to sanitise, and the difference is
// what is on the other side of the guard. There, a NaN belongs to a slot, and
// the answer is to skip the slot, because a fader pulled to zero is how an
// operator escapes broken material and it has to work on exactly the material
// that is broken (ADR-0042). Here the value belongs to a **history**: nothing
// renders it, no fader is under it, and a NaN read back into the frame it came
// from is written back into the history on the same frame — so it outlives its
// cause and there is nothing to skip.
//
// **Colour only.** Alpha is coverage and a retention does not change what a
// frame covers, so the source's alpha goes through unchanged.
@fragment
fn fs_keep(in: VsOut) -> @location(0) vec4<f32> {
    let s = textureLoad(src, vec2<i32>(in.clip.xy), 0);
    let v = s.rgb;
    let kept = select(vec3<f32>(0.0), v, (v == v) & (abs(v) < vec3<f32>(3.4e38)));
    return vec4<f32>(kept, s.a);
}
