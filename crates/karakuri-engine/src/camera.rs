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
/// a handheld rig mixed at 0.3, which is what `docs/ir-spec.md` means by
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
    /// **The three numbers that say where this camera is standing, as a node's
    /// parameters** — the name and the declared range of each, in the order a
    /// surface draws them.
    ///
    /// **The built-in camera is a node with no procedure behind it, so this is
    /// the `param` list it would have had.** A procedure states its own names
    /// and ranges and the Set reads them off the artifact; there is no artifact
    /// here, so the engine states them, and everything downstream follows from
    /// the map being populated rather than from a rule about cameras: a
    /// `--param L3:0:radius`, a published control a knob is learned against, a
    /// `bind`, an authority, a ride carried across a rebuild. Nothing in this
    /// crate special-cases a camera to make any of that work.
    ///
    /// **The ranges of the first two are `docs/ir-spec.md`'s own**, from the
    /// `sweep` example under *The `camera` block (L3)* — the simplest L3
    /// anybody would write, which is this orbit spelled as a procedure and
    /// declares `radius : float [1.0, 40.0]` and `speed : float [0.0, 2.0]`.
    /// Taking them from there rather than inventing two is what keeps the
    /// built-in and the procedure that replaces it the same camera to a hand.
    /// `height` has no such precedent — the example hard-codes 2.0 — and its
    /// range is derived rather than chosen: it is symmetric because looking up
    /// from underneath is as much a shot as looking down, and it reaches as far
    /// either way as `radius` does, past which the eye is further from the
    /// target than any radius could put it, which is a distance and not a
    /// height.
    ///
    /// **The other three are not here**, and that is the decision rather than
    /// an omission: `fov_y`, `near` and `far` are what the projection *is*
    /// rather than where the camera is, and `blend weighted` normalises every
    /// fragment's depth against the near and far planes — so a fader on either
    /// would move how the picture composites while appearing to move the
    /// camera, which is a control whose effect is not the one it draws. A hand
    /// that wants them writes an L3, which declares whatever it likes.
    /// `docs/adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md`.
    pub const PLACEMENT: [(&'static str, [f32; 2]); 3] = [
        ("radius", [1.0, 40.0]),
        ("speed", [0.0, 2.0]),
        ("height", [-40.0, 40.0]),
    ];

    /// What this orbit holds under one of [`Orbit::PLACEMENT`]'s names, or
    /// `None` for a name that is not one of the three.
    ///
    /// **A read by name because the map is keyed by name**, and the two have to
    /// agree; a caller that matched on the field would be a second copy of this
    /// table's spelling.
    pub fn placement(&self, key: &str) -> Option<f32> {
        match key {
            "radius" => Some(self.radius),
            "speed" => Some(self.speed),
            "height" => Some(self.height),
            _ => None,
        }
    }

    /// Set one of the three by name. `false` for a name that is not one of
    /// them, which is a caller writing a lens number through the placement
    /// door.
    pub fn set_placement(&mut self, key: &str, value: f32) -> bool {
        match key {
            "radius" => self.radius = value,
            "speed" => self.speed = value,
            "height" => self.height = value,
            _ => return false,
        }
        true
    }

    /// This orbit's three, as the parameter map of the node it produces.
    ///
    /// **The defaults are this orbit's fields and not [`Orbit::default`]'s**,
    /// because a Set built from a file that recorded a camera is built with
    /// that camera: what the declaration *is* for this node is what the record
    /// said, exactly as a procedure's declaration is what its file said.
    pub fn placement_values(&self) -> Vec<(String, f32)> {
        Orbit::PLACEMENT
            .iter()
            .filter_map(|(key, _)| Some((key.to_string(), self.placement(key)?)))
            .collect()
    }

    /// The three declared ranges, as the range map of the node it produces.
    pub fn placement_ranges() -> Vec<(String, [f32; 2])> {
        Orbit::PLACEMENT
            .iter()
            .map(|(key, range)| (key.to_string(), *range))
            .collect()
    }

    /// **This orbit with its three placement numbers taken from `param`** —
    /// the lens three left as they are.
    ///
    /// What the built-in camera node's producer is every frame: the values live
    /// in the node's parameter map, so what a hand moved, what a binding is
    /// blending and what a rebuild carried are all already in the number this
    /// is handed. `param` returning `None` leaves the field alone, which is
    /// what a Set whose map has not got the key would want and is unreachable
    /// where this crate builds the map.
    pub fn with_placement(&self, param: impl Fn(&str) -> Option<f32>) -> Orbit {
        let mut out = *self;
        for (key, _) in Orbit::PLACEMENT {
            if let Some(value) = param(key) {
                out.set_placement(key, value);
            }
        }
        out
    }

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
        [
            self.near,
            1.0 / (self.far - self.near).max(f32::MIN_POSITIVE),
        ]
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
        assert!(
            (clip[0] / clip[3]).abs() < 1e-5,
            "x = {}",
            clip[0] / clip[3]
        );
        assert!(
            (clip[1] / clip[3]).abs() < 1e-5,
            "y = {}",
            clip[1] / clip[3]
        );
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
