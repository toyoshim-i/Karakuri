//! The built-in camera.
//!
//! L3 is a slot in the design but not in V1, so until it exists the camera is a
//! built-in driven by a `camera` record. Matrices are column-major with a 0..1
//! depth range, which is what wgpu expects.

pub type Mat4 = [[f32; 4]; 4];

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

impl Orbit {
    /// `t` is simulation time, never wall clock.
    pub fn view_proj(&self, t: f32, aspect: f32) -> Mat4 {
        let a = t * self.speed * std::f32::consts::TAU;
        let eye = [self.radius * a.cos(), self.height, self.radius * a.sin()];
        mul(
            perspective_rh(self.fov_y, aspect, self.near, self.far),
            look_at_rh(eye, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        )
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
