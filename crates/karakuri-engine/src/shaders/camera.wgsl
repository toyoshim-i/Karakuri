// Deriving the forms a renderer reads from the six numbers a camera is.
//
// **One invocation, once per frame.** There is nothing to parallelise: this is
// a handful of cross products and a 4x4 multiply. It is a compute pass rather
// than host arithmetic because the state it reads may have been written by an
// L3 that followed an element, and reading that back to the host is the one
// thing this architecture is built to avoid.
//
// The arithmetic is `karakuri_engine::camera::State`'s, transcribed. That the
// two agree is asserted by a test that reads this buffer back, not assumed —
// the host copy still drives the debug overlay, so they are two derivations of
// one camera and the usual hazard applies.

{{STATE_STRUCT}}
{{CAMERA_STRUCT}}

// The aspect ratio, which belongs to the canvas and not to the camera — which
// is why it arrives here, at the derivation, rather than in the state above.
// Three scalars of padding rather than a `vec3`, which would align to 16 and
// make this 32 bytes for one number.
struct Canvas {
    aspect: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<storage, read> state: CameraState;
@group(0) @binding(1) var<storage, read_write> derived: Camera;
@group(0) @binding(2) var<uniform> canvas: Canvas;

@compute @workgroup_size(1)
fn derive() {
    let eye = state.eye;
    let f = normalize(state.look_at - eye);
    let s = normalize(cross(f, state.up));
    let u = cross(s, f);

    // Right-handed look-at, column-major: each `vec4` below is a column.
    let view = mat4x4<f32>(
        vec4<f32>(s.x, u.x, -f.x, 0.0),
        vec4<f32>(s.y, u.y, -f.y, 0.0),
        vec4<f32>(s.z, u.z, -f.z, 0.0),
        vec4<f32>(-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0),
    );

    // Right-handed perspective onto a 0..1 depth range, which is what wgpu
    // expects.
    let near = state.near;
    let far = state.far;
    let half = tan(state.fov_y * 0.5);
    let k = 1.0 / half;
    let proj = mat4x4<f32>(
        vec4<f32>(k / canvas.aspect, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, k, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, far / (near - far), -1.0),
        vec4<f32>(0.0, 0.0, (near * far) / (near - far), 0.0),
    );

    derived.view_proj = proj * view;
    derived.eye = eye;
    derived.fwd = f;
    // Pre-scaled, so nothing downstream has to know what the projection was.
    derived.right = s * (half * canvas.aspect);
    derived.up = u * half;
    // A frustum of zero depth would divide by zero here and hand every
    // weighted fragment a NaN weight; the floor is the smallest positive
    // normal, matching the host's `State::depth_range`.
    derived.depth_range = vec2<f32>(near, 1.0 / max(far - near, 1.17549435e-38));
}

// WebXR: one eye riding this camera. `head` is the eye's view matrix relative
// to where the headset started (rig space -> eye space) and `proj` is the
// eye's own, possibly off-centre, projection. Both column-major.
struct Xr {
    head: mat4x4<f32>,
    proj: mat4x4<f32>,
};
@group(0) @binding(3) var<uniform> xr: Xr;

// The same placement `derive` reads — whatever the orbit or an L3 wrote this
// frame — taken as a rig the performer rides, with the head's own motion
// composed on top: the camera carries them, and they look around from it.
@compute @workgroup_size(1)
fn derive_xr() {
    let eye = state.eye;
    let f = normalize(state.look_at - eye);
    let s = normalize(cross(f, state.up));
    let u = cross(s, f);
    let rig = mat4x4<f32>(
        vec4<f32>(s.x, u.x, -f.x, 0.0),
        vec4<f32>(s.y, u.y, -f.y, 0.0),
        vec4<f32>(s.z, u.z, -f.z, 0.0),
        vec4<f32>(-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0),
    );
    let view = xr.head * rig;

    // WebXR projections map depth onto -1..1, the GL convention; wgpu clips
    // to 0..1, so remap z' = (z + w) / 2.
    let to_unit_depth = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.5, 0.0),
        vec4<f32>(0.0, 0.0, 0.5, 1.0),
    );
    derived.view_proj = to_unit_depth * xr.proj * view;

    // The view's rotation rows are the eye's axes in world space; its
    // position is the rotation's transpose applied to minus the translation.
    let axes = transpose(mat3x3<f32>(view[0].xyz, view[1].xyz, view[2].xyz));
    derived.eye = -(axes * view[3].xyz);
    derived.fwd = -axes[2];
    // Pre-scaled like `derive`'s: by the half-extent of the frustum, which a
    // projection carries as the reciprocal of its diagonal. An off-centre
    // frustum's shift is not carried — a marching L4 is centred on the eye's
    // forward axis (the celestial dome is where that gets answered).
    derived.right = axes[0] / xr.proj[0][0];
    derived.up = axes[1] / xr.proj[1][1];
    let near = state.near;
    let far = state.far;
    derived.depth_range = vec2<f32>(near, 1.0 / max(far - near, 1.17549435e-38));
}
