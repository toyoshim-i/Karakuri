//! Camera representation and orbit projection mathematics.
//!
//! Defines [`State`] for world-space camera parameters, [`Orbit`] for orbital trajectories,
//! and projection utilities producing column-major matrices with `[0, 1]` depth ranges for WebGPU.

pub type Mat4 = [[f32; 4]; 4];

/// World-space camera state parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub fov_y: f32,
    /// Near and far clipping planes (used by weighted transparency to normalize fragment depth).
    pub near: f32,
    pub far: f32,
}

/// Orbital camera trajectory controller.
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

/// Camera position and pre-scaled ray basis vectors for ray-marching shaders.
#[derive(Debug, Clone, Copy)]
pub struct Basis {
    pub eye: [f32; 3],
    pub forward: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
}

impl Orbit {
    /// Placement parameter definitions and valid ranges (`radius`, `speed`, `height`).
    pub const PLACEMENT: [(&'static str, [f32; 2]); 3] = [
        ("radius", [1.0, 40.0]),
        ("speed", [0.0, 2.0]),
        ("height", [-40.0, 40.0]),
    ];

    /// Returns the parameter value for the given placement key, or `None`.
    pub fn placement(&self, key: &str) -> Option<f32> {
        match key {
            "radius" => Some(self.radius),
            "speed" => Some(self.speed),
            "height" => Some(self.height),
            _ => None,
        }
    }

    /// Sets a placement parameter by name, returning `true` on success.
    pub fn set_placement(&mut self, key: &str, value: f32) -> bool {
        match key {
            "radius" => self.radius = value,
            "speed" => self.speed = value,
            "height" => self.height = value,
            _ => return false,
        }
        true
    }

    /// Returns the current values of all placement parameters.
    pub fn placement_values(&self) -> Vec<(String, f32)> {
        Orbit::PLACEMENT
            .iter()
            .filter_map(|(key, _)| Some((key.to_string(), self.placement(key)?)))
            .collect()
    }

    /// Returns the declared valid ranges for all placement parameters.
    pub fn placement_ranges() -> Vec<(String, [f32; 2])> {
        Orbit::PLACEMENT
            .iter()
            .map(|(key, range)| (key.to_string(), *range))
            .collect()
    }

    /// Returns a copy of this orbit updated with values from the given parameter lookup function.
    pub fn with_placement(&self, param: impl Fn(&str) -> Option<f32>) -> Orbit {
        let mut out = *self;
        for (key, _) in Orbit::PLACEMENT {
            if let Some(value) = param(key) {
                out.set_placement(key, value);
            }
        }
        out
    }

    /// Computes camera state at simulation time `t` (in seconds).
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

    /// Computes camera position and ray basis vectors at simulation time `t` for `aspect`.
    pub fn basis(&self, t: f32, aspect: f32) -> Basis {
        self.state(t).basis(aspect)
    }

    /// Computes the view-projection matrix at simulation time `t` for `aspect`.
    pub fn view_proj(&self, t: f32, aspect: f32) -> Mat4 {
        self.state(t).view_proj(aspect)
    }
}

impl State {
    /// Computes camera position and ray basis vectors for the given aspect ratio.
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

    /// Computes the view-projection matrix for the given aspect ratio.
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        mul(
            perspective_rh(self.fov_y, aspect, self.near, self.far),
            look_at_rh(self.eye, self.target, self.up),
        )
    }

    /// Returns `(near, 1 / (far - near))` for depth weighting in fragment shaders.
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
