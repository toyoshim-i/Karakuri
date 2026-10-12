---
id: ir-spec
title: Karakuri IR Specification
summary: Specification of the .kir plain-text shader procedure language, execution algebra, type contracts, ambients, and built-in functions.
---

# Karakuri IR Specification (v0.2)

Extension `.kir`. Plain text. Written by LLMs, validated by the compiler, lowered to WGSL.

One `.kir` file defines one procedure (`proc`). A Set is an assembly of procedures: one or more `L1` simulation sources, optional `L2` deforms, `L3` cameras, and `L4` renderers, routed into an `L5` master chain.

---

## 1. File Structure

```
proc <name> {
  <header declarations>

  <block_name> {
    <statements>
  }
}
```

Every procedure declares its `kind` and relevant parameters in the header, followed by its execution block.

---

## 2. Layer Kinds & Algebra

Karakuri defines six procedure kinds with fixed semantic signatures:

```
L1    : ()                 -> Geometry (generates particles, meshes, or ribbons)
L2    : Geometry           -> Geometry (modulates or deforms geometry)
L3    : ()                 -> Camera   (defines viewpoint, position, and projection)
L4    : (Geometry, Camera) -> Texture  (rasterizes geometry into pixels)
Field : vec3               -> float    (evaluates a 3D Signed Distance Field)
L5    : [Texture]          -> Texture  (post-processing and screen-space frame effects)
```

### Required Declarations & Blocks per Kind

| Kind | Required Header Declarations | Required Block | Execution Target |
|---|---|---|---|
| **`L1`** | `kind L1`, `capacity`, `topology` | `element { ... }` | Compute pass (per-element simulation) |
| **`L2`** | `kind L2` | `deform { ... }` | Compute pass (per-element modification) |
| **`L3`** | `kind L3` | `camera { ... }` | Evaluated once per frame |
| **`L4`** | `kind L4`, `blend` | `fragment { ... }` | Render pass (vertex + fragment shaders) |
| **`Field`**| `kind Field` | `field { ... }` | Spliced WGSL function (evaluated anywhere) |
| **`L5`** | `kind L5` | `frame { ... }` | Fullscreen render pass |

---

## 3. Header Declarations

### `topology` (`L1` only)
Specifies the geometric representation emitted by the simulation:
- `points`: Sprites (one quad per element).
- `lines`: Segments (one line per element; requires `clip` and `clip_b` in paired L4).
- `triangles`: Direct vertex-shader-art triangle mesh (3 elements per triangle face; static sources only).
- `grid`: 2D tessellated quad grid mesh with automated index buffer (for terrain/cloth; static sources only).
- `ribbon`: Continuous connected quad-strip ribbon (for trails/flowlines; static sources only).

### `capacity` (`L1` only)
Defines the memory allocation range and default element count:
```
capacity [min, max] = default
```
Example:
```
capacity [65536, 1048576] = 262144
```

### `blend` (`L4` only)
Defines how fragment colors combine with underlying surfaces:
- `additive`: Color sums without occlusion. Alpha acts as emission intensity and can exceed `1.0`.
- `weighted`: Order-independent transparency (occludes by alpha `[0.0, 1.0]`).
- `opaque`: Depth-tested and depth-writing (early-Z occlusion against Depth32 buffer).

### Fullscreen `L4` Procedures & Built-in VR Projection
An `L4` procedure that declares no `vertex` block acts as a **fullscreen fragment renderer** (e.g. 2D procedural patterns, raymarchers, canvas backgrounds). Fullscreen procedures cannot declare `consumes` because there are no elements to read from.
- On 2D screens: Renders across the entire display quad.
- In WebXR Immersive mode: Automatically projected via **Built-in VR Projection** (`Wall`, `Dome`, or `Kaleidosky`). In `Dome` mode, `point_coord` maps the zenith $(0.5, 0.5)$ directly overhead down to $-30^\circ$ below the horizon.
- For complete coordinate mapping contracts, WGSL examples, and celestial dome shader guidelines, read [VR Projection](vr-projection).

### `param` (All kinds)
Declares an interactive, controllable parameter exposed to the console, MIDI, and LLM:
```
param name : type [min, max] = default
```
Example:
```
param speed : float [0.0, 5.0] = 1.0
param radius : float [0.1, 10.0] = 2.5
```

### `emit` and `consumes`
Establishes the typed attribute contract between simulation (`L1`/`L2`) and rendering (`L4`):
- `emit`: Attributes written by `L1` or `L2` (e.g. `emit position, velocity, tint, age`).
- `consumes`: Attributes read by downstream nodes (`L2` or `L4`). Must be a subset of emitted attributes: `consumes ⊆ emit`.

Built-in element attributes:
- `position` (`vec3`): World-space element coordinates.
- `velocity` (`vec3`): World-space velocity vector.
- `normal` (`vec3`): Surface normal vector.
- `uv` (`vec2`): Texture parameter coordinates.
- `age` (`float`): Lifetime elapsed since spawn in seconds.
- `size` (`float`): Particle or line scale factor.
- `tint` (`vec3`): Base color tint (linear RGB).

*Note*: Element random seed `seed` is an ambient value, carried automatically and readable anywhere in element pipelines. It must never be declared in `emit` or `consumes`.

### `uses`
Declares dependency wiring between nodes:
- `uses far : Geometry` (`L2` or `L3`): Second geometry slot for morphing/blending, or subject geometry tracked by camera.
- `uses shape : Field` (`L1`, `L2`, `L3`, `L4`): References a `Field` procedure for collision or raymarching.
- `uses view : Camera` (`L4` only): Binds an explicit `L3` procedural camera.
- `uses id : Source` (`L1`, `L2`, `L4`): References a source identity slot for instance masking.
- `uses under : Texture` (`L5` only): References another texture layer for compositing.

---

## 4. Built-in Environment & Signals

Available implicitly inside execution blocks as ambient values:

| Name | Type | Description |
|---|---|---|
| `t` | `float` | Continuous elapsed session time in seconds |
| `beats` | `float` | Musical beat clock on the session tempo grid |
| `dt` | `float` | Time elapsed since previous frame in seconds (L1, L3, L5) |
| `seed` | `uint` | Carried per-element pseudo-random seed (L1, L2, L4) |
| `source` | `uint` | Identifier / salt of the geometry source instance (L1, L2, L4) |
| `capacity` | `uint` | Element allocation capacity of the L1 source (L1) |
| `copy` | `uint` | Copy index produced by upstream amplifying L2 node (L2, L4) |
| `point` | `vec3` | World-space evaluation point in Field procedures (Field) |
| `camera` | `mat4` | View-projection matrix in L4 vertex stage (L4) |
| `point_coord` | `vec2` | Normalized fragment or frame coordinates `[0.0, 1.0]^2` (L4 fragment, L5 frame) |
| `eye` | `vec3` | World-space camera eye position (fullscreen L4 fragment) |
| `ray` | `vec3` | World-space unit ray direction (fullscreen L4 fragment) |

*Signals*: Dynamic control signals (such as audio FFT bands or MIDI CC values) are not directly readable from IR. Instead, declare an interactive `param` and bind the signal to that parameter in the Set configuration.
*VR Projection*: In WebXR immersive mode, fullscreen fragment procedures render to the virtual screen or dome. See [VR Projection](vr-projection) for coordinate mapping and projection modes.

---

## 5. Built-in Functions

Function signatures and shapes are strictly defined by the compiler checker. The complete generated reference is published at [IR Vocabulary](ir-vocabulary) (see `crates/karakuri-ir/src/builtin.rs`).

Commonly used built-in functions:
- **Math**: `min`, `max`, `clamp`, `abs`, `sign`, `floor`, `ceil`, `fract`, `mod`, `sqrt`, `pow`, `exp`, `log`, `step`, `smoothstep`, `mix`.
- **Trigonometry**: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`.
- **Vector**: `dot`, `cross`, `length`, `distance`, `normalize`, `reflect`, `refract`.
- **Noise & Hash**: `hash1(seed)`, `hash2(seed)`, `hash3(seed)`, `value_noise(p)`, `perlin(p)`, `simplex(p)`, `fbm(p, octaves)`, `curl(p)`.
- **SDF Primitives & Operators**: `sd_sphere(p, r)`, `sd_box(p, b)`, `sd_torus(p, t)`, `sd_plane(p, n, d)`, `op_union(d1, d2)`, `op_smooth_union(d1, d2, k)`, `op_subtract(d1, d2)`.
- **Color**: `hsv_to_rgb(hsv)`, `rgb_to_hsv(rgb)`, `srgb_to_linear(rgb)`, `linear_to_srgb(rgb)`.
- **Transform**: `rot_x(p, angle)`, `rot_y(p, angle)`, `rot_z(p, angle)`, `rot_axis(p, axis, angle)`, `sphere_point(u, v)`.

---

## 6. Complete Examples

### Minimal Particle Simulation (`L1`)
```kir
proc particle_fountain {
  kind L1
  topology points
  capacity [1024, 65536] = 16384

  param spread : float [0.1, 4.0] = 1.2
  param gravity : float [0.0, 20.0] = 9.8

  emit position, velocity, tint, age

  element {
    tint = hsv_to_rgb(vec3(fract(beats * 0.1 + hash1(seed) * 0.2), 0.8, 1.0));
    if (age <= 0.0 || position.y < -5.0) {
      // Spawn new particle
      let angle = hash1(seed) * 6.2831853;
      let speed = spread * (0.5 + 0.5 * hash1(seed + 1u));
      velocity = vec3(cos(angle) * speed, 5.0 + 2.0 * hash1(seed + 2u), sin(angle) * speed);
      position = vec3(0.0, 0.0, 0.0);
      age = 3.0 + hash1(seed + 3u);
    } else {
      // Update simulation step
      velocity = velocity - vec3(0.0, gravity * dt, 0.0);
      position = position + velocity * dt;
      age = age - dt;
    }
  }
}
```

### Paired Particle Renderer (`L4`)
```kir
proc particle_glow {
  kind L4
  blend additive

  param point_size : float [0.001, 0.08] = 0.015

  consumes position, tint, age

  vertex {
    // Project 3D position to clip space
    clip = camera * vec4(position, 1.0);
    point_rate = point_size;
  }

  fragment {
    // Radial soft particle falloff
    let dist = length(point_coord * 2.0 - 1.0);
    let alpha = max(0.0, 1.0 - dist * dist) * clamp(age, 0.0, 1.0);
    color = vec4(tint * 1.5, alpha);
  }
}
```
