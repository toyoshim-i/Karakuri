//! The builtin function library, emitted as WGSL source.
//!
//! Most builtins in `karakuri_ir::builtin` are WGSL builtins under the same
//! name (`abs`, `pow`, `normalize`, `atan2`, …) and need no helper at all —
//! [`crate::lower`] calls them by [`Builtin::name`] directly. This module
//! only has to supply the ones WGSL does not have: hashing and noise, SDF,
//! rotation, the distribution and colour functions, and the `mod` wrapper.
//! Every one of those is written under the *same* name the IR builtin table
//! uses, so call-site lowering never special-cases which kind of name it is
//! emitting — with one exception, `mod` on floats, documented at
//! [`mod_helper_name`].
//!
//! Only what a procedure actually calls is emitted, tracked via
//! [`Requirements`] — see the `mod_helper_appears_only_when_used` test in
//! `lib.rs` for why that matters.
//!
//! # The seed salt
//!
//! Every seeded builtin (`Builtin::is_seeded`) bottoms out in `hash1`, and
//! `hash1` reads `u.seed_salt` straight from the module-scope uniform
//! binding rather than taking it as a parameter. That works because both
//! [`crate::l1`] and [`crate::l4`] name their uniform binding `u` and give
//! it a `seed_salt: u32` field — the same trick `points.wgsl` already uses.
//! It means salting never has to be threaded through call sites; it is
//! simply in scope, exactly as the spec requires ("the salt must be in scope
//! wherever they are emitted").
//!
//! # Noise fidelity
//!
//! `value_noise`, `perlin`, `simplex`, and `curl` are original, self
//! consistent implementations built on one hashing primitive — deterministic
//! given the seed stream, and syntactically/semantically valid WGSL, which
//! is what this crate is checked against. They are not attempts at
//! reference-quality noise (`simplex` in particular is gradient noise at an
//! offset, not a real simplex lattice — see its doc comment). Getting the
//! *visual* character of these right is future tuning work, not a lowering
//! concern.

use std::collections::HashSet;

use karakuri_ir::builtin::Builtin;
use karakuri_ir::Ty;

/// Which optional helpers a generated shader needs, accumulated while
/// lowering its blocks.
///
/// `HashSet` rather than `BTreeSet`: neither `Builtin` nor `Ty` implements
/// `Ord` (they are plain enums in `karakuri-ir`, ordered only by identity),
/// and adding it there is out of scope for this crate. Emission order below
/// does not depend on set-iteration order — every helper is looked up by
/// `contains`, not walked — so this costs nothing but determinism of
/// incidental whitespace between two unrelated helpers, which no test here
/// relies on.
#[derive(Debug, Default)]
pub struct Requirements {
    pub builtins: HashSet<Builtin>,
    /// Types `mod` (the builtin or `%`) was used at. `Ty` is always one of
    /// `Float`, `Vec2`, `Vec3`, `Vec4` here — `Mod`'s domain is
    /// `FloatOrVector` and `%` on `int`/`uint` lowers to the native
    /// operator, never through here.
    pub mod_types: HashSet<Ty>,
}

impl Requirements {
    pub fn note_builtin(&mut self, b: Builtin) {
        self.builtins.insert(b);
    }

    pub fn note_mod(&mut self, ty: Ty) {
        self.mod_types.insert(ty);
    }

    /// Take on another module's requirements.
    ///
    /// **For a spliced field**, whose body lives in the caller's module and
    /// therefore needs the caller's prelude to carry its helpers. Without this
    /// a field calling `sd_torus` produces a call to a function nothing
    /// emitted, in a shader that checked clean — the prelude being
    /// demand-driven is exactly what makes the omission silent.
    pub fn absorb(&mut self, other: &Requirements) {
        self.builtins.extend(other.builtins.iter().copied());
        self.mod_types.extend(other.mod_types.iter().copied());
    }
}

/// The helper function name for `mod`/`%` at type `ty`.
///
/// This is the one builtin whose call-site name is not [`Builtin::name`]
/// verbatim: WGSL has no function named `mod`, and `%` on `f32` does not have
/// `mod`'s semantics — `a - b * floor(a / b)` takes the sign of the
/// **divisor** where WGSL's `%` takes the dividend's, so lowering must insert
/// a wrapper. Every other
/// helper in this module is named identically to the `Builtin` it
/// implements, so a call site can always emit `func.name()(args)` — except
/// this one, which is why lowering special-cases exactly one variant instead
/// of guessing from the function's own name.
pub fn mod_helper_name(ty: Ty) -> &'static str {
    match ty {
        Ty::Float => "mod_f32",
        Ty::Vec2 => "mod_vec2",
        Ty::Vec3 => "mod_vec3",
        Ty::Vec4 => "mod_vec4",
        other => unreachable!("mod is not defined at type {other:?}"),
    }
}

/// **`frame_step` — a distance as a fraction of the frame's height, in the
/// coordinates `tap` takes.**
///
/// `master.wgsl`'s `step_uv` with the same arithmetic and one fewer thing
/// exposed: `.y` is the fraction itself, and `.x` is that fraction scaled by
/// the aspect ratio so the displacement is isotropic **in texels** at any
/// aspect ratio. The size comes from `u.viewport`, which is the frame's own,
/// and it is deliberately not reachable any other way — no ambient carries the
/// render size, because the render size is not part of the picture and one
/// frame is rendered at the largest enabled output's size and scaled into the
/// rest. So the conversion is performed without the number being handed over,
/// and a `.kir` has no way to write a radius in texels.
///
/// Named for the builtin like every other helper here, under
/// [`crate::lower::mangle_local`]'s namespace rule: nothing this crate emits
/// can be captured by a name a procedure can spell.
const FRAME_STEP: &str = "\
// A distance in fractions of the frame's height, in `tap`'s coordinates —
// isotropic in texels at any aspect ratio. The size is the frame's own and is
// never handed to the procedure.
fn frame_step(r: f32) -> vec2<f32> {
    return vec2<f32>(r * u.viewport.y / u.viewport.x, r);
}
";

fn mod_source(ty: Ty) -> String {
    let t = crate::ty::wgsl_ty(ty);
    let name = mod_helper_name(ty);
    format!(
        "// IR `mod`/`%` semantics: always the sign of `b`, unlike WGSL's `%`.\n\
         fn {name}(a: {t}, b: {t}) -> {t} {{ return a - b * floor(a / b); }}\n"
    )
}

const HASH1: &str = "\
// PCG-derived integer hash, salted with this layer's seed stream value so
// re-seeding a Set changes randomness without touching structure.
fn hash1(x: u32) -> f32 {
    var h: u32 = (x ^ u.seed_salt) * 747796405u + 2891336453u;
    h = ((h >> ((h >> 28u) + 4u)) ^ h) * 277803737u;
    h = (h >> 22u) ^ h;
    return f32(h) * (1.0 / 4294967295.0);
}
";

const HASH2: &str = "\
fn hash2(x: u32) -> vec2<f32> {
    return vec2<f32>(hash1(x), hash1(x ^ 0x9e3779b9u));
}
";

const HASH3: &str = "\
fn hash3(x: u32) -> vec3<f32> {
    return vec3<f32>(hash1(x), hash1(x ^ 0x9e3779b9u), hash1(x ^ 0x85ebca6bu));
}
";

const LATTICE_HASH: &str = "\
// Folds an integer lattice coordinate into hash1's domain, so every noise
// primitive below shares one salted hashing core instead of each inventing
// its own.
fn lattice_hash(p: vec3<i32>) -> f32 {
    let ux = bitcast<u32>(p.x) * 374761393u;
    let uy = bitcast<u32>(p.y) * 668265263u;
    let uz = bitcast<u32>(p.z) * 2147483647u;
    return hash1(ux ^ uy ^ uz);
}
";

const LATTICE_GRADIENT: &str = "\
fn lattice_gradient(p: vec3<i32>) -> vec3<f32> {
    let a = lattice_hash(p);
    let b = lattice_hash(p + vec3<i32>(97, 57, 13));
    let theta = a * 6.2831853;
    let z = b * 2.0 - 1.0;
    let r = sqrt(max(0.0, 1.0 - z * z));
    return vec3<f32>(r * cos(theta), r * sin(theta), z);
}
";

const VALUE_NOISE: &str = "\
fn value_noise(p: vec3<f32>) -> f32 {
    let i = vec3<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let c000 = lattice_hash(i + vec3<i32>(0, 0, 0));
    let c100 = lattice_hash(i + vec3<i32>(1, 0, 0));
    let c010 = lattice_hash(i + vec3<i32>(0, 1, 0));
    let c110 = lattice_hash(i + vec3<i32>(1, 1, 0));
    let c001 = lattice_hash(i + vec3<i32>(0, 0, 1));
    let c101 = lattice_hash(i + vec3<i32>(1, 0, 1));
    let c011 = lattice_hash(i + vec3<i32>(0, 1, 1));
    let c111 = lattice_hash(i + vec3<i32>(1, 1, 1));

    let x00 = mix(c000, c100, u.x);
    let x10 = mix(c010, c110, u.x);
    let x01 = mix(c001, c101, u.x);
    let x11 = mix(c011, c111, u.x);
    let y0 = mix(x00, x10, u.y);
    let y1 = mix(x01, x11, u.y);
    return mix(y0, y1, u.z) * 2.0 - 1.0;
}
";

const PERLIN: &str = "\
fn perlin(p: vec3<f32>) -> f32 {
    let i = vec3<i32>(floor(p));
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    let g000 = lattice_gradient(i + vec3<i32>(0, 0, 0));
    let g100 = lattice_gradient(i + vec3<i32>(1, 0, 0));
    let g010 = lattice_gradient(i + vec3<i32>(0, 1, 0));
    let g110 = lattice_gradient(i + vec3<i32>(1, 1, 0));
    let g001 = lattice_gradient(i + vec3<i32>(0, 0, 1));
    let g101 = lattice_gradient(i + vec3<i32>(1, 0, 1));
    let g011 = lattice_gradient(i + vec3<i32>(0, 1, 1));
    let g111 = lattice_gradient(i + vec3<i32>(1, 1, 1));

    let n000 = dot(g000, f - vec3<f32>(0.0, 0.0, 0.0));
    let n100 = dot(g100, f - vec3<f32>(1.0, 0.0, 0.0));
    let n010 = dot(g010, f - vec3<f32>(0.0, 1.0, 0.0));
    let n110 = dot(g110, f - vec3<f32>(1.0, 1.0, 0.0));
    let n001 = dot(g001, f - vec3<f32>(0.0, 0.0, 1.0));
    let n101 = dot(g101, f - vec3<f32>(1.0, 0.0, 1.0));
    let n011 = dot(g011, f - vec3<f32>(0.0, 1.0, 1.0));
    let n111 = dot(g111, f - vec3<f32>(1.0, 1.0, 1.0));

    let x00 = mix(n000, n100, u.x);
    let x10 = mix(n010, n110, u.x);
    let x01 = mix(n001, n101, u.x);
    let x11 = mix(n011, n111, u.x);
    let y0 = mix(x00, x10, u.y);
    let y1 = mix(x01, x11, u.y);
    return mix(y0, y1, u.z);
}
";

const SIMPLEX: &str = "\
// A pragmatic stand-in, not a reference simplex lattice: real 3D simplex
// noise needs a skewed lattice and a permutation table, machinery this
// codegen path does not need to carry for correctness (naga validation is
// the bar, not noise fidelity). Reuses perlin's gradient field at an offset
// so `simplex` and `perlin` decorrelate rather than being the same curve.
fn simplex(p: vec3<f32>) -> f32 {
    return perlin(p * 1.3737 + vec3<f32>(19.19, 7.7, 3.3));
}
";

const CURL: &str = "\
// Curl of a vector potential built from three decorrelated offsets of the
// same gradient noise, via central differences.
fn curl(p: vec3<f32>) -> vec3<f32> {
    let e = 0.1;
    let dx = vec3<f32>(e, 0.0, 0.0);
    let dy = vec3<f32>(0.0, e, 0.0);
    let dz = vec3<f32>(0.0, 0.0, e);

    let off_x = vec3<f32>(91.7, 3.3, 47.1);
    let off_y = vec3<f32>(13.1, 71.9, 5.5);
    let off_z = vec3<f32>(57.3, 29.7, 83.1);

    let d_pz_dy = (perlin(p + off_z + dy) - perlin(p + off_z - dy)) / (2.0 * e);
    let d_py_dz = (perlin(p + off_y + dz) - perlin(p + off_y - dz)) / (2.0 * e);
    let d_px_dz = (perlin(p + off_x + dz) - perlin(p + off_x - dz)) / (2.0 * e);
    let d_pz_dx = (perlin(p + off_z + dx) - perlin(p + off_z - dx)) / (2.0 * e);
    let d_py_dx = (perlin(p + off_y + dx) - perlin(p + off_y - dx)) / (2.0 * e);
    let d_px_dy = (perlin(p + off_x + dy) - perlin(p + off_x - dy)) / (2.0 * e);

    return vec3<f32>(d_pz_dy - d_py_dz, d_px_dz - d_pz_dx, d_py_dx - d_px_dy);
}
";

const SD_SPHERE: &str = "\
fn sd_sphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}
";

const SD_BOX: &str = "\
fn sd_box(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}
";

const SD_TORUS: &str = "\
fn sd_torus(p: vec3<f32>, t: vec2<f32>) -> f32 {
    let q = vec2<f32>(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}
";

const SD_PLANE: &str = "\
fn sd_plane(p: vec3<f32>, n: vec3<f32>, h: f32) -> f32 {
    return dot(p, n) + h;
}
";

const OP_UNION: &str = "\
fn op_union(a: f32, b: f32) -> f32 { return min(a, b); }
";

const OP_SMOOTH_UNION: &str = "\
fn op_smooth_union(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}
";

const OP_SUBTRACT: &str = "\
fn op_subtract(a: f32, b: f32) -> f32 { return max(a, -b); }
";

const OP_INTERSECT: &str = "\
fn op_intersect(a: f32, b: f32) -> f32 { return max(a, b); }
";

const ROT_X: &str = "\
fn rot_x(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(p.x, c * p.y - s * p.z, s * p.y + c * p.z);
}
";

const ROT_Y: &str = "\
fn rot_y(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);
}
";

const ROT_Z: &str = "\
fn rot_z(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * p.x - s * p.y, s * p.x + c * p.y, p.z);
}
";

const ROT_AXIS: &str = "\
fn rot_axis(p: vec3<f32>, axis: vec3<f32>, a: f32) -> vec3<f32> {
    let k = normalize(axis);
    let c = cos(a);
    let s = sin(a);
    return p * c + cross(k, p) * s + k * dot(k, p) * (1.0 - c);
}
";

const SPHERE_POINT: &str = "\
fn sphere_point(a: f32, b: f32) -> vec3<f32> {
    let z = 1.0 - 2.0 * a;
    let r = sqrt(max(0.0, 1.0 - z * z));
    let phi = 6.2831853 * b;
    return vec3<f32>(r * cos(phi), r * sin(phi), z);
}
";

const DISC_POINT: &str = "\
fn disc_point(a: f32, b: f32) -> vec2<f32> {
    let r = sqrt(a);
    let theta = 6.2831853 * b;
    return vec2<f32>(r * cos(theta), r * sin(theta));
}
";

const SRGB_TO_LINEAR: &str = "\
fn srgb_to_linear_scalar(c: f32) -> f32 {
    if (c <= 0.04045) { return c / 12.92; }
    return pow((c + 0.055) / 1.055, 2.4);
}
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        srgb_to_linear_scalar(c.x),
        srgb_to_linear_scalar(c.y),
        srgb_to_linear_scalar(c.z),
    );
}
";

const LINEAR_TO_SRGB: &str = "\
fn linear_to_srgb_scalar(c: f32) -> f32 {
    if (c <= 0.0031308) { return c * 12.92; }
    return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}
fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_to_srgb_scalar(c.x),
        linear_to_srgb_scalar(c.y),
        linear_to_srgb_scalar(c.z),
    );
}
";

const HSV_TO_RGB: &str = "\
// Returns LINEAR rgb: the sRGB->linear conversion happens inside, via
// srgb_to_linear, so callers get the hue they expect without thinking about
// colour space. Do not swap this for linear_to_srgb -- that compiles fine
// and is wrong, invisibly, until it is on screen.
fn hsv_to_rgb(c: vec3<f32>) -> vec3<f32> {
    let h = fract(c.x) * 6.0;
    let s = clamp(c.y, 0.0, 1.0);
    let v = clamp(c.z, 0.0, 1.0);
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
    return srgb_to_linear(srgb);
}
";

const RGB_TO_HSV: &str = "\
// Inverse of hsv_to_rgb: undoes the linearisation first, since hsv_to_rgb's
// output (and this function's input) is linear RGB.
fn rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let srgb = linear_to_srgb(c);
    let mx = max(srgb.x, max(srgb.y, srgb.z));
    let mn = min(srgb.x, min(srgb.y, srgb.z));
    let d = mx - mn;
    var h = 0.0;
    if (d > 0.00001) {
        var hh: f32;
        if (mx == srgb.x) {
            hh = (srgb.y - srgb.z) / d;
        } else if (mx == srgb.y) {
            hh = (srgb.z - srgb.x) / d + 2.0;
        } else {
            hh = (srgb.x - srgb.y) / d + 4.0;
        }
        hh = hh / 6.0;
        h = hh - floor(hh);
    }
    let s = select(0.0, d / mx, mx > 0.00001);
    return vec3<f32>(h, s, mx);
}
";

/// Emits every helper `req` actually needs, in dependency order, as one
/// WGSL source blob to splice after the uniform struct and before the
/// entry points.
pub fn render(req: &Requirements) -> String {
    let mut out = String::new();

    for &ty in &req.mod_types {
        out.push_str(&mod_source(ty));
        out.push('\n');
    }

    let b = &req.builtins;
    let uses = |f: fn(Builtin) -> bool| b.iter().copied().any(f);

    let need_hash1 = uses(|x| x.is_seeded()) || b.contains(&Builtin::Hash1);
    let need_lattice = uses(|x| {
        matches!(
            x,
            Builtin::ValueNoise | Builtin::Perlin | Builtin::Simplex | Builtin::Fbm | Builtin::Curl
        )
    });
    let need_gradient = uses(|x| {
        matches!(
            x,
            Builtin::Perlin | Builtin::Simplex | Builtin::Fbm | Builtin::Curl
        )
    });
    let need_perlin = need_gradient; // everything that needs the gradient lattice calls perlin for it

    // **First, because nothing else calls it and it reads only the uniform.**
    // `texel` and `tap` have no helper — they lower straight to
    // `textureLoad`/`textureSampleLevel` at the call site — so this is the
    // whole of what the three L5 builtins ask the prelude for.
    if b.contains(&Builtin::FrameStep) {
        out.push_str(FRAME_STEP);
        out.push('\n');
    }
    if need_hash1 {
        out.push_str(HASH1);
        out.push('\n');
    }
    if b.contains(&Builtin::Hash2) {
        out.push_str(HASH2);
        out.push('\n');
    }
    if b.contains(&Builtin::Hash3) {
        out.push_str(HASH3);
        out.push('\n');
    }
    if need_lattice {
        out.push_str(LATTICE_HASH);
        out.push('\n');
    }
    if b.contains(&Builtin::ValueNoise) {
        out.push_str(VALUE_NOISE);
        out.push('\n');
    }
    if need_gradient {
        out.push_str(LATTICE_GRADIENT);
        out.push('\n');
    }
    if need_perlin {
        out.push_str(PERLIN);
        out.push('\n');
    }
    if b.contains(&Builtin::Simplex) {
        out.push_str(SIMPLEX);
        out.push('\n');
    }
    if b.contains(&Builtin::Curl) {
        out.push_str(CURL);
        out.push('\n');
    }
    // Fbm is unrolled inline at call sites (see crate::lower), never called
    // as a function, so it needs perlin in scope but contributes no helper
    // of its own.

    if b.contains(&Builtin::SdSphere) {
        out.push_str(SD_SPHERE);
        out.push('\n');
    }
    if b.contains(&Builtin::SdBox) {
        out.push_str(SD_BOX);
        out.push('\n');
    }
    if b.contains(&Builtin::SdTorus) {
        out.push_str(SD_TORUS);
        out.push('\n');
    }
    if b.contains(&Builtin::SdPlane) {
        out.push_str(SD_PLANE);
        out.push('\n');
    }
    if b.contains(&Builtin::OpUnion) {
        out.push_str(OP_UNION);
        out.push('\n');
    }
    if b.contains(&Builtin::OpSmoothUnion) {
        out.push_str(OP_SMOOTH_UNION);
        out.push('\n');
    }
    if b.contains(&Builtin::OpSubtract) {
        out.push_str(OP_SUBTRACT);
        out.push('\n');
    }
    if b.contains(&Builtin::OpIntersect) {
        out.push_str(OP_INTERSECT);
        out.push('\n');
    }

    if b.contains(&Builtin::RotX) {
        out.push_str(ROT_X);
        out.push('\n');
    }
    if b.contains(&Builtin::RotY) {
        out.push_str(ROT_Y);
        out.push('\n');
    }
    if b.contains(&Builtin::RotZ) {
        out.push_str(ROT_Z);
        out.push('\n');
    }
    if b.contains(&Builtin::RotAxis) {
        out.push_str(ROT_AXIS);
        out.push('\n');
    }

    if b.contains(&Builtin::SpherePoint) {
        out.push_str(SPHERE_POINT);
        out.push('\n');
    }
    if b.contains(&Builtin::DiscPoint) {
        out.push_str(DISC_POINT);
        out.push('\n');
    }

    // Colour helpers have a dependency in each direction, so pull both
    // conversion functions in whenever either public entry point is used.
    let need_srgb_to_linear = b.contains(&Builtin::HsvToRgb) || b.contains(&Builtin::SrgbToLinear);
    let need_linear_to_srgb = b.contains(&Builtin::RgbToHsv) || b.contains(&Builtin::LinearToSrgb);
    if need_srgb_to_linear {
        out.push_str(SRGB_TO_LINEAR);
        out.push('\n');
    }
    if need_linear_to_srgb {
        out.push_str(LINEAR_TO_SRGB);
        out.push('\n');
    }
    if b.contains(&Builtin::HsvToRgb) {
        out.push_str(HSV_TO_RGB);
        out.push('\n');
    }
    if b.contains(&Builtin::RgbToHsv) {
        out.push_str(RGB_TO_HSV);
        out.push('\n');
    }

    out
}
