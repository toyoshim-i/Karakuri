//! The built-in camera, and what a camera *is*.
//!
//! [`State`] is the edge: `eye`, `target`, `up`, `fov_y`, `near`, `far` — the six
//! numbers `docs/ir-spec.md` settles on, for the reason it gives, which is
//! multiplicity. A lerp of two view-projection matrices is not a projection of
//! anything, so what crosses between a camera and a renderer is state and the
//! derived forms are produced from it.
//!
//! [`Orbit`] is the built-in **producer** of that state — a `camera` record, and
//! the only one there is until an L3 procedure can be one. It has no derivations
//! of its own beyond [`Orbit::state`]; `view_proj` and `basis` are [`State`]'s.
//!
//! **The derivation that reaches a renderer is not this one.** An L4 reads a GPU
//! buffer written by `shaders/camera.wgsl`, because an L3 that follows an element
//! cannot be evaluated on the host without a readback — see
//! [`crate::node::Camera`]. What is here is the same arithmetic in Rust, for the
//! overlay in [`crate::points`] and for a test to hold the shader against; that
//! the two agree is asserted rather than assumed.
//!
//! Matrices are column-major with a 0..1 depth range, which is what wgpu expects.

pub type Mat4 = [[f32; 4]; 4];

/// The six numbers a camera is, in world space.
///
/// **Not a matrix, and deliberately.** Blending two trajectories — an orbit and
/// a handheld rig mixed at 0.3, which is what `docs/roadmap.md` means by
/// L3-multiple — is meaningful on these and meaningless on the matrices derived
/// from them.
///
/// **Aspect ratio is not here.** It belongs to the canvas, not to the camera,
/// which is why both derivations below take it as an argument.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov_y: f32,
    /// **Not decorative.** `blend weighted` normalises a fragment's depth
    /// against these two planes, so a camera that moves `far` moves every
    /// weighted fragment's weight with it.
    pub near: f32,
    pub far: f32,
}

/// `{"t":"camera","kind":"orbit","radius":8.0,"speed":0.15}`
#[derive(Debug, Clone, Copy)]
pub struct Orbit {
    pub radius: f32,
    /// Revolutions per second of simulation time.
    pub speed: f32,
    pub height: f32,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Orbit {
    fn default() -> Orbit {
        Orbit {
            radius: 8.0,
            speed: 0.15,
            height: 2.0,
            fov_y: std::f32::consts::FRAC_PI_3,
            near: 0.1,
            far: 100.0,
        }
    }
}

/// Where the camera is and the three vectors a ray through a pixel is built
/// from, in world space.
///
/// **`right` and `up` are pre-scaled** by the field of view and the aspect
/// ratio, so a fragment's ray is `normalize(forward + right * ndc.x + up *
/// ndc.y)` and nothing downstream has to know what the projection was. That is
/// the whole reason this exists rather than an inverse view-projection matrix:
/// the convention stays here, where the camera is, instead of being restated in
/// every procedure that marches — and a `mat4` inverse stays out of a language
/// that has no operator for one.
#[derive(Debug, Clone, Copy)]
pub struct Basis {
    pub eye: [f32; 3],
    pub forward: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
}

impl Orbit {
    /// Where this orbit is at this instant. `t` is simulation time, never wall
    /// clock.
    ///
    /// **The only thing an `Orbit` computes.** Everything a renderer reads comes
    /// out of the [`State`] this returns, so an orbit and an L3 that produced
    /// the same six numbers are indistinguishable downstream — which is what
    /// makes replacing this with a procedure a change of producer rather than a
    /// change of edge.
    pub fn state(&self, t: f32) -> State {
        let a = t * self.speed * std::f32::consts::TAU;
        State {
            eye: [self.radius * a.cos(), self.height, self.radius * a.sin()],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov_y: self.fov_y,
            near: self.near,
            far: self.far,
        }
    }

    /// The eye, and the ray basis at this instant. `t` is simulation time.
    pub fn basis(&self, t: f32, aspect: f32) -> Basis {
        self.state(t).basis(aspect)
    }

    /// `t` is simulation time, never wall clock.
    pub fn view_proj(&self, t: f32, aspect: f32) -> Mat4 {
        self.state(t).view_proj(aspect)
    }
}

impl State {
    /// The ray basis: the eye, and the three vectors a ray through a pixel is
    /// built from.
    ///
    /// Derived from the same three lines [`State::view_proj`] uses, so the
    /// marched picture and the rasterized one are looking from the same place. A
    /// second derivation would be two cameras that agree until one of them is
    /// edited.
    pub fn basis(&self, aspect: f32) -> Basis {
        let forward = normalize(sub(self.target, self.eye));
        let right = normalize(cross(forward, self.up));
        let up = cross(right, forward);
        let half = (self.fov_y * 0.5).tan();
        Basis {
            eye: self.eye,
            forward,
            right: scale(right, half * aspect),
            up: scale(up, half),
        }
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        mul(
            perspective_rh(self.fov_y, aspect, self.near, self.far),
            look_at_rh(self.eye, self.target, self.up),
        )
    }

    /// `(near, 1 / (far - near))`, which is what a weighted fragment is measured
    /// against — packed so the shader multiplies rather than divides.
    pub fn depth_range(&self) -> [f32; 2] {
        [self.near, 1.0 / (self.far - self.near).max(f32::MIN_POSITIVE)]
    }
}

fn perspective_rh(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y * 0.5).tan();
    let mut m = [[0.0; 4]; 4];
    m[0][0] = f / aspect;
    m[1][1] = f;
    m[2][2] = far / (near - far);
    m[2][3] = -1.0;
    m[3][2] = (near * far) / (near - far);
    m
}

fn scale(v: [f32; 3], k: f32) -> [f32; 3] {
    [v[0] * k, v[1] * k, v[2] * k]
}

fn look_at_rh(eye: [f32; 3], centre: [f32; 3], up: [f32; 3]) -> Mat4 {
    let f = normalize(sub(centre, eye));
    let s = normalize(cross(f, up));
    let u = cross(s, f);
    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0],
    ]
}

/// Column-major product: `result[c][r] = sum_k a[k][r] * b[c][k]`.
fn mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut out = [[0.0; 4]; 4];
    for (c, col) in out.iter_mut().enumerate() {
        for (r, cell) in col.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    out
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = dot(v, v).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transform(m: Mat4, p: [f32; 3]) -> [f32; 4] {
        let mut out = [0.0; 4];
        for (r, cell) in out.iter_mut().enumerate() {
            *cell = m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2] + m[3][r];
        }
        out
    }

    #[test]
    fn the_origin_lands_in_the_middle_of_the_screen() {
        let cam = Orbit {
            height: 0.0,
            ..Orbit::default()
        };
        let clip = transform(cam.view_proj(0.0, 16.0 / 9.0), [0.0, 0.0, 0.0]);
        assert!(clip[3] > 0.0, "origin must be in front of the camera");
        assert!((clip[0] / clip[3]).abs() < 1e-5, "x = {}", clip[0] / clip[3]);
        assert!((clip[1] / clip[3]).abs() < 1e-5, "y = {}", clip[1] / clip[3]);
    }

    #[test]
    fn depth_maps_into_zero_to_one() {
        let cam = Orbit::default();
        let m = cam.view_proj(0.0, 1.0);
        // A point at the origin sits between the near and far planes.
        let clip = transform(m, [0.0, 0.0, 0.0]);
        let ndc_z = clip[2] / clip[3];
        assert!((0.0..=1.0).contains(&ndc_z), "ndc z = {ndc_z}");
    }

    #[test]
    fn the_camera_orbits() {
        let cam = Orbit::default();
        let a = cam.view_proj(0.0, 1.0);
        let b = cam.view_proj(1.0 / cam.speed / 4.0, 1.0);
        assert_ne!(a, b, "a quarter revolution must change the view");
    }

    #[test]
    fn a_full_revolution_returns_to_the_start() {
        let cam = Orbit::default();
        let a = cam.view_proj(0.0, 1.0);
        let b = cam.view_proj(1.0 / cam.speed, 1.0);
        for c in 0..4 {
            for r in 0..4 {
                assert!((a[c][r] - b[c][r]).abs() < 1e-3, "[{c}][{r}]");
            }
        }
    }
}
