//! Synthetic procedures and shader sources used to dial ladder rungs.

pub(crate) fn field_lens_src(steps: u32) -> String {
    format!(
        r#"
proc field_lens {{
  kind  L4
  blend additive

  uses shape : Field

  param spin     : float [0.0, 2.0] = 0.3
  param exposure : float [0.0, 8.0] = 1.0
  param glow     : float [0.0, 4.0] = 0.8

  fragment {{
    let a  = -t * spin;
    var p  = rot_y(eye, a);
    let rd = rot_y(ray, a);

    var d  = 10.0;
    var it = 0.0;
    var hit = 0.0;

    for i in 0..{steps} {{
      d = shape(p);
      if d < 0.005 {{
        hit = 1.0;
      }}
      if hit < 0.5 {{
        it = it + 1.0;
      }}
      p = p + rd * max(d, 0.005);
    }}

    let near = 1.0 - it / {steps}.0;
    let g    = hit * pow(near, 1.5) * exposure;

    color = vec4(g * 0.55 + hit * glow * 0.05, g * 0.3, g * 1.1, 1.0);
  }}
}}
"#
    )
}

/// `field_march.kir`, with the cheap sphere-only normal replaced by a six-tap
/// central difference over the whole field — the "six more field evaluations
/// [that] would fix it and do not fit under the ceiling".
pub(crate) fn field_march_src(correct_normal: bool) -> String {
    const SDF: &str = "op_union(op_smooth_union(sd_sphere(q - vec3(0.9, 0.0, 0.0), ball), \
                       sd_box(q + vec3(0.9, 0.0, 0.0), vec3(box, box, box)), blend_k), \
                       sd_torus(q, vec2(ring, thick)))";
    let normal = if correct_normal {
        format!(
            r#"
    let e = 0.001;
    var q = p + vec3(e, 0.0, 0.0);
    let nx0 = {SDF};
    q = p - vec3(e, 0.0, 0.0);
    let nx1 = {SDF};
    q = p + vec3(0.0, e, 0.0);
    let ny0 = {SDF};
    q = p - vec3(0.0, e, 0.0);
    let ny1 = {SDF};
    q = p + vec3(0.0, 0.0, e);
    let nz0 = {SDF};
    q = p - vec3(0.0, 0.0, e);
    let nz1 = {SDF};
    let n = normalize(vec3(nx0 - nx1, ny0 - ny1, nz0 - nz1));
"#
        )
    } else {
        "    let n = normalize(p - vec3(0.9, 0.0, 0.0));\n".to_string()
    };
    format!(
        r#"
proc field_march {{
  kind  L4
  blend additive

  param ball     : float [0.2, 4.0]  = 1.6
  param box      : float [0.2, 4.0]  = 1.2
  param ring     : float [0.2, 4.0]  = 2.6
  param thick    : float [0.05, 1.0] = 0.35
  param blend_k   : float [0.0, 1.5] = 0.55
  param spin      : float [0.0, 2.0] = 0.35
  param exposure  : float [0.0, 8.0] = 1.0
  param glow      : float [0.0, 4.0] = 0.7

  fragment {{
    let a  = -t * spin;
    var p  = rot_y(eye, a);
    let rd = rot_y(ray, a);

    var d  = 10.0;
    var it = 0.0;

    for i in 0..48 {{
      let sph = sd_sphere(p - vec3(0.9, 0.0, 0.0), ball);
      let bx  = sd_box(p + vec3(0.9, 0.0, 0.0), vec3(box, box, box));
      d = op_union(op_smooth_union(sph, bx, blend_k), sd_torus(p, vec2(ring, thick)));

      if d < 0.004 {{
        it = 1.0;
      }}
      p = p + rd * max(d, 0.004);
    }}

{normal}
    let key = max(0.0, dot(n, normalize(vec3(0.5, 0.8, 0.3))));
    let rim = pow(1.0 - max(0.0, dot(n, -rd)), 3.0);

    let c = hsv_to_rgb(vec3(0.58 + rim * 0.25, 0.7, 1.0));

    color = vec4(c * (key + rim * glow) * exposure * it, it);
  }}
}}
"#
    )
}

/// `soft_points`, with the velocity term dropped from `point_rate` so coverage
/// is exactly `capacity * (point_scale * height)^2` and computable, and with a
/// padding loop in `fragment` as the dial. The loop carries a real dependency
/// on `d` so nothing folds it away.
pub(crate) fn pad_points_src(pad: u32) -> String {
    let body = if pad == 0 {
        String::new()
    } else {
        format!(
            "    var acc = 0.0;\n    for i in 0..{pad} {{\n      \
             acc = acc * 1.000001 + d * 0.5;\n    }}\n"
        )
    };
    let use_acc = if pad == 0 {
        "0.0".to_string()
    } else {
        "acc * 0.000000001".to_string()
    };
    format!(
        r#"
proc pad_points {{
  kind  L4
  blend additive

  consumes position, velocity, age

  param point_scale : float [0.00069, 0.0556] = 0.00556
  param hue         : float [0.0, 1.0]  = 0.58
  param spread      : float [0.0, 1.0]  = 0.30
  param exposure    : float [0.0, 8.0]  = 1.00
  param falloff     : float [0.5, 8.0]  = 3.0

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_rate = point_scale;
  }}

  fragment {{
    let d = length(point_coord * 2.0 - 1.0);
{body}
    let a = pow(max(0.0, 1.0 - d + {use_acc}), falloff);
    let c = hsv_to_rgb(vec3(hue + hash1(seed) * spread, 0.75, 1.0));
    color = vec4(c * exposure, a);
  }}
}}
"#
    )
}

pub(crate) fn pad_shell_src(pad: u32) -> String {
    let body = if pad == 0 {
        String::new()
    } else {
        format!(
            "    var extra = vec3(0.0, 0.0, 0.0);\n    for i in 0..{pad} {{\n      \
             extra = curl(extra * 0.001 + base * 0.4);\n    }}\n"
        )
    };
    let use_extra = if pad == 0 {
        "vec3(0.0, 0.0, 0.0)".to_string()
    } else {
        "extra * 0.000001".to_string()
    };
    format!(
        r#"
proc pad_shell {{
  kind     L1
  topology points
  capacity [4096, 1048576] = 262144

  param radius     : float [0.1, 8.0] = 2.6
  param turbulence : float [0.0, 3.0] = 1.1
  param swirl      : float [0.0, 2.0] = 0.35
  param drift      : float [0.0, 2.0] = 0.6

  emit position, velocity, age

  element {{
    let u    = hash1(seed);
    let v    = hash1(seed + 1000u);
    let base = sphere_point(u, v) * radius;

    let phase = t * drift + hash1(seed + 7u) * 6.2831853;
    let flow  = curl(base * 0.4 + vec3(0.0, t * 0.15, 0.0)) * turbulence;
    let p     = rot_y(base + flow * 0.35, t * swirl + sin(phase) * 0.2);

{body}
    position = p + {use_extra};
    velocity = flow;
    age      = age + dt;
  }}
}}
"#
    )
}

/// A per-element L4 that costs as little as a renderer can. **Not fullscreen:**
/// `Set::step` skips the simulation entirely when every renderer is fullscreen,
/// so a fullscreen L4 would measure an element ladder that never ran.
pub(crate) const TINY_DOTS: &str = r#"
proc tiny_dots {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.0007;
  }

  fragment {
    color = vec4(0.01, 0.01, 0.01, 1.0);
  }
}
"#;

pub(crate) fn pad_fountain_src(pad: u32) -> String {
    let body = if pad == 0 {
        String::new()
    } else {
        format!(
            "    var extra = vec3(0.0, 0.0, 0.0);\n    for i in 0..{pad} {{\n      \
             extra = curl(extra * 0.001 + vec3(u, v, w));\n    }}\n"
        )
    };
    let use_extra = if pad == 0 {
        "vec3(0.0, 0.0, 0.0)".to_string()
    } else {
        "extra * 0.000001".to_string()
    };
    format!(
        r#"
proc pad_fountain {{
  kind     L1
  topology points
  capacity [4096, 1048576] = 262144

  param spawn_rate : float [0.0, 60000.0] = 26000.0
  param lifetime   : float [0.5, 12.0]    = 5.0
  param nozzle     : float [0.02, 1.5]    = 0.30
  param launch     : float [0.5, 8.0]     = 4.4
  param gravity    : float [0.0, 12.0]    = 3.0
  param turbulence : float [0.0, 3.0]     = 0.55
  param drag       : float [0.9, 1.0]     = 0.998

  emit position, velocity, age

  spawn {{
    let u = hash1(seed);
    let v = hash1(seed + 1u);
    let w = hash1(seed + 2u);

    let d = disc_point(u, v) * nozzle;

{body}
    position = vec3(d.x, -0.9, d.y) + {use_extra};
    velocity = vec3(d.x * 2.4, launch * (0.75 + 0.5 * w), d.y * 2.4);
    age      = 0.0;
  }}

  element {{
    let flow = curl(position * 0.6 + vec3(0.0, t * 0.3, 0.0)) * turbulence;
    let v    = (velocity + (flow + vec3(0.0, 0.0 - gravity, 0.0)) * dt) * drag;

    velocity = v;
    position = position + v * dt;
    age      = age + dt;

    if position.y < -1.0 {{
      kill();
    }}
    if age > lifetime {{
      kill();
    }}
  }}
}}
"#
    )
}
