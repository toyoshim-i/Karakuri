// A hardcoded stand-in for what karakuri-codegen will emit.
//
// This is the `soft_points` example of the spec fed by a spawn-less L1: every
// element is live from frame zero and `seed` is its slot index. Nothing here is
// meant to outlive the code generator. It exists so that the render path can be
// built, measured, and kept working before the IR can produce anything -- the
// triangle that must never break.
//
// Two things here are lowering decisions, not conveniences:
//
// * WebGPU has no point size. `PrimitiveTopology::PointList` always rasterises
//   a single pixel, so `topology points` expands each element into a quad: six
//   vertices per instance, with the corner in `vertex_index` and the element in
//   `instance_index`. `point_coord` falls out of the corner.
// * The hash builtins are salted with the layer's seed stream value, so
//   re-seeding a Set changes its randomness without touching structure.

struct Uniforms {
    view_proj: mat4x4<f32>,
    viewport: vec2<f32>,
    t: f32,
    dt: f32,
    capacity: u32,
    seed_salt: u32,
    point_scale: f32,
    hue: f32,
    exposure: f32,
    falloff: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

// PCG-derived integer hash. Deterministic, no state, no time.
fn hash1(x: u32) -> f32 {
    var h: u32 = (x ^ u.seed_salt) * 747796405u + 2891336453u;
    h = ((h >> ((h >> 28u) + 4u)) ^ h) * 277803737u;
    h = (h >> 22u) ^ h;
    return f32(h) * (1.0 / 4294967295.0);
}

fn sphere_point(a: f32, b: f32) -> vec3<f32> {
    let z = 1.0 - 2.0 * a;
    let r = sqrt(max(0.0, 1.0 - z * z));
    let phi = 6.2831853 * b;
    return vec3<f32>(r * cos(phi), r * sin(phi), z);
}

// Returns linear RGB: the sRGB->linear conversion happens inside, so that
// authors get the hue they expect without thinking about colour space.
fn hsv_to_rgb(c: vec3<f32>) -> vec3<f32> {
    let h = fract(c.x) * 6.0;
    let s = c.y;
    let v = c.z;
    let i = floor(h);
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    var srgb: vec3<f32>;
    switch (i32(i) % 6) {
        case 0: { srgb = vec3<f32>(v, t, p); }
        case 1: { srgb = vec3<f32>(q, v, p); }
        case 2: { srgb = vec3<f32>(p, v, t); }
        case 3: { srgb = vec3<f32>(p, q, v); }
        case 4: { srgb = vec3<f32>(t, p, v); }
        default: { srgb = vec3<f32>(v, p, q); }
    }
    // sRGB -> linear, the cheap approximation. The exact transfer function
    // lands with the code generator.
    return pow(srgb, vec3<f32>(2.2));
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) point_coord: vec2<f32>,
    @location(1) @interpolate(flat) seed: u32,
    @location(2) speed: f32,
};

// The six corners of a quad, in [0, 1].
fn corner_of(i: u32) -> vec2<f32> {
    switch i {
        case 0u: { return vec2<f32>(0.0, 0.0); }
        case 1u: { return vec2<f32>(1.0, 0.0); }
        case 2u: { return vec2<f32>(0.0, 1.0); }
        case 3u: { return vec2<f32>(0.0, 1.0); }
        case 4u: { return vec2<f32>(1.0, 0.0); }
        default: { return vec2<f32>(1.0, 1.0); }
    }
}

@vertex
fn vs(@builtin(vertex_index) corner: u32, @builtin(instance_index) seed: u32) -> VsOut {
    // What a spawn-less L1 element block would produce: a shell that drifts.
    let base = sphere_point(hash1(seed), hash1(seed + 1000u));
    let wobble = 0.35 * sin(u.t * 0.6 + hash1(seed + 7u) * 6.2831853);
    let world = base * (2.0 + wobble);
    let velocity = base * 0.6 * cos(u.t * 0.6 + hash1(seed + 7u) * 6.2831853);

    var out: VsOut;
    let centre = u.view_proj * vec4<f32>(world, 1.0);

    // Quad expansion in clip space, so the sprite keeps its pixel size
    // regardless of depth.
    let c = corner_of(corner) * 2.0 - 1.0;
    let size = u.point_scale * (0.3 + 0.7 * clamp(length(velocity) * 0.5, 0.0, 1.0));
    let offset = c * size / u.viewport * centre.w;

    out.clip = vec4<f32>(centre.xy + offset, centre.zw);
    out.point_coord = corner_of(corner);
    out.seed = seed;
    out.speed = length(velocity);
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let d = length(in.point_coord * 2.0 - 1.0);
    let a = pow(max(0.0, 1.0 - d), u.falloff);
    let c = hsv_to_rgb(vec3<f32>(u.hue + hash1(in.seed) * 0.05, 0.7, 1.0));
    // Linear, straight alpha. Values above 1.0 are expected -- they are what
    // will feed bloom.
    return vec4<f32>(c * u.exposure, a);
}
