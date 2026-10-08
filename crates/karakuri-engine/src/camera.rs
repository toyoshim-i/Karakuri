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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orbit {
    pub radius: f32,
    /// Revolutions per second of simulation time.
    pub speed: f32,
    pub height: f32,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    /// Horizontal azimuth angle offset (turntable angle, in radians).
    pub yaw: f32,
    /// Vertical target look-at offset.
    pub target_y: f32,
    /// Camera roll angle around the forward look-at axis (Dutch angle, in radians).
    pub roll: f32,
    /// Vertical harmonic oscillation amplitude (bobbing).
    pub bob: f32,
    /// Distance modulation offset from radius (breathing / zoom pulse).
    pub dolly: f32,
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
            yaw: 0.0,
            target_y: 0.0,
            roll: 0.0,
            bob: 0.0,
            dolly: 0.0,
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
    /// Placement parameter definitions and valid ranges.
    pub const PLACEMENT: [(&'static str, [f32; 2]); 9] = [
        ("radius", [1.0, 40.0]),
        ("speed", [0.0, 2.0]),
        ("height", [-40.0, 40.0]),
        ("fov_y", [0.2, 2.5]),
        ("yaw", [-std::f32::consts::PI, std::f32::consts::PI]),
        ("target_y", [-20.0, 20.0]),
        ("roll", [-std::f32::consts::PI, std::f32::consts::PI]),
        ("bob", [0.0, 10.0]),
        ("dolly", [-20.0, 20.0]),
    ];

    /// Returns the parameter value for the given placement key, or `None`.
    pub fn placement(&self, key: &str) -> Option<f32> {
        match key {
            "radius" => Some(self.radius),
            "speed" => Some(self.speed),
            "height" => Some(self.height),
            "fov_y" => Some(self.fov_y),
            "yaw" => Some(self.yaw),
            "target_y" => Some(self.target_y),
            "roll" => Some(self.roll),
            "bob" => Some(self.bob),
            "dolly" => Some(self.dolly),
            _ => None,
        }
    }

    /// Sets a placement parameter by name, returning `true` on success.
    pub fn set_placement(&mut self, key: &str, value: f32) -> bool {
        match key {
            "radius" => self.radius = value,
            "speed" => self.speed = value,
            "height" => self.height = value,
            "fov_y" => self.fov_y = value,
            "yaw" => self.yaw = value,
            "target_y" => self.target_y = value,
            "roll" => self.roll = value,
            "bob" => self.bob = value,
            "dolly" => self.dolly = value,
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
        let a = t * self.speed * std::f32::consts::TAU + self.yaw;
        let r = (self.radius + self.dolly).max(0.1);
        let h = self.height
            + if self.bob.abs() > 1e-5 {
                (t * self.speed.max(0.05) * std::f32::consts::TAU * 2.0).sin() * self.bob
            } else {
                0.0
            };
        let eye = [r * a.cos(), h, r * a.sin()];
        let target = [0.0, self.target_y, 0.0];
        let fwd = normalize(sub(target, eye));
        let world_up = [0.0, 1.0, 0.0];
        let right = if fwd[0].abs() < 1e-4 && fwd[2].abs() < 1e-4 {
            [1.0, 0.0, 0.0]
        } else {
            normalize(cross(fwd, world_up))
        };
        let base_up = cross(right, fwd);
        let up = if self.roll.abs() > 1e-5 {
            let cos_r = self.roll.cos();
            let sin_r = self.roll.sin();
            add(scale(base_up, cos_r), scale(right, sin_r))
        } else {
            base_up
        };
        State {
            eye,
            target,
            up,
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

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
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
    if len > 1e-6 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Configuration for VR projection modes (Wall, Dome, Kaleidosky) on 2D fullscreen shaders.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VrConfig {
    /// 0.0 = Wall (default infinite wall screen), 1.0 = Dome, 2.0 = Kaleidosky
    pub mode: f32,
    /// Radial N-division for concentric rings in Kaleidosky (default 4.0)
    pub rings: f32,
    /// Azimuthal M-division for radial symmetry in Kaleidosky (default 6.0)
    pub facets: f32,
    /// 1.0 = ping-pong mirror reflection, 0.0 = cyclic repeat (default 1.0)
    pub mirror: f32,
    /// Scale zoom factor (default 1.0)
    pub zoom: f32,
}

impl Default for VrConfig {
    fn default() -> Self {
        Self {
            mode: 0.0,
            rings: 4.0,
            facets: 6.0,
            mirror: 1.0,
            zoom: 1.0,
        }
    }
}

/// One WebXR eye, as the headset reports it: column-major view and projection.
///
/// `view` maps rig space to eye space, where rig space is the headset's own
/// space re-centred on where it started; the engine composes it with each
/// camera's placement on the GPU, so the eye rides the camera
/// (`derive_xr` in camera.wgsl). `proj` is the eye's own, off-centre,
/// GL-convention projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StereoMatrices {
    pub view: Mat4,
    pub proj: Mat4,
    pub vr_config: VrConfig,
}

impl StereoMatrices {
    /// Takes WebXR's column-major 4x4 view and projection arrays.
    pub fn from_slices(view: &[f32; 16], proj: &[f32; 16]) -> Self {
        let to_mat4 = |s: &[f32; 16]| -> Mat4 {
            let mut m = [[0.0; 4]; 4];
            for (c, column) in m.iter_mut().enumerate() {
                column.copy_from_slice(&s[c * 4..c * 4 + 4]);
            }
            m
        };
        StereoMatrices {
            view: to_mat4(view),
            proj: to_mat4(proj),
            vr_config: VrConfig::default(),
        }
    }

    /// Sets the VR projection config.
    pub fn with_vr_config(mut self, config: VrConfig) -> Self {
        self.vr_config = config;
        self
    }
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
