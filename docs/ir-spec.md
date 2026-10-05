# Karakuri IR Specification (v0.2)

Extension `.kir`. Plain text. Written by LLMs, validated by the compiler, lowered to WGSL.

One `.kir` file defines one procedure (`proc`). A Set is an assembly of procedures: one or more `L1` simulation sources, optional `L2` deforms, `L3` cameras, and `L4` renderers, routed into an `L5` master chain.

---

## 1. File Structure

```kir
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
| **`L2`** | `kind L2`, `uses <name> : Geometry` | `deform { ... }` | Compute pass (per-element modification) |
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
```kir
capacity [65536, 1048576] = 262144
```

### `blend` (`L4` only)
Defines how fragment colors combine with underlying surfaces:
- `additive`: Color sums without occlusion. Alpha acts as emission intensity and can exceed `1.0`.
- `weighted`: Order-independent transparency (occludes by alpha `[0.0, 1.0]`).
- `opaque`: Depth-tested and depth-writing (early-Z occlusion against Depth32 buffer).

### `param` (All kinds)
Declares an interactive, controllable parameter exposed to the console, MIDI, and LLM:
```kir
param speed : float = 1.0 [0.0, 5.0]
param radius : float = 2.5 [0.1, 10.0]
```

### `emit` and `consumes`
Establishes the typed attribute contract between simulation (`L1`/`L2`) and rendering (`L4`):
- `emit`: Attributes written by `L1` or `L2` (e.g. `emit pos, vel, col, uv, age`).
- `consumes`: Attributes read by `L4` (must be a subset of emitted attributes: `consumes ⊆ emit`).

Built-in attributes: `pos` (vec3), `vel` (vec3), `col` (vec4), `uv` (vec2), `size` (float), `age` (float), `seed` (uint), `rot` (vec4 quaternion).

### `uses`
Declares dependency wiring between nodes:
- `uses far : Geometry` (`L2` only): Second geometry slot for morphing/blending.
- `uses shape : Field` (`L1`, `L2`, `L3`, `L4`): References a `Field` procedure for collision or raymarching.
- `uses view : Camera` (`L4` only): Binds an explicit `L3` procedural camera.
- `uses under : Texture` (`L5` only): References another texture layer for compositing.

---

## 4. Built-in Environment & Signals

Available implicitly inside execution blocks:

| Name | Type | Description |
|---|---|---|
| `time` | `float` | Continuous elapsed session time in seconds |
| `beat` | `float` | Musical beat clock (accumulated continuous beats according to tempo) |
| `dt` | `float` | Time elapsed since previous frame in seconds |
| `aspect` | `float` | Target aspect ratio (`width / height`) |
| `resolution` | `vec2` | Target dimensions in physical pixels |
| `pointer` | `vec2` | Normalized pointer coordinates `[0.0, 1.0]` |
| `audio` | `vec4` | Instantaneous audio frequency energy: `(sub_bass, bass, mid, high)` |

---

## 5. Built-in Functions

- **Math**: `min`, `max`, `clamp`, `abs`, `sign`, `floor`, `ceil`, `fract`, `mod`, `sqrt`, `pow`, `exp`, `log`, `step`, `smoothstep`, `mix`.
- **Trigonometry**: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`.
- **Vector**: `dot`, `cross`, `length`, `distance`, `normalize`, `reflect`, `refract`.
- **Noise & Hash**:
  - `hash(uint) -> float`
  - `noise2d(vec2) -> float`
  - `noise3d(vec3) -> float`
  - `fbm2d(vec2, int octaves) -> float`
  - `fbm3d(vec3, int octaves) -> float`
- **SDF Primitives**: `sdf_sphere(p, r)`, `sdf_box(p, b)`, `sdf_torus(p, t)`, `sdf_cylinder(p, h, r)`.
- **Color**: `hsv2rgb(vec3) -> vec3`, `rgb2hsv(vec3) -> vec3`.

---

## 6. Complete Examples

### Minimal Particle Simulation (`L1`)
```kir
proc particle_fountain {
  kind L1
  topology points
  capacity [1024, 65536] = 16384

  param spread : float = 1.2 [0.1, 4.0]
  param gravity : float = 9.8 [0.0, 20.0]

  emit pos, vel, col, age

  element {
    if (age <= 0.0 || pos.y < -5.0) {
      // Spawn new particle
      let angle = hash(seed) * 6.28318;
      let speed = spread * (0.5 + 0.5 * hash(seed + 1u));
      vel = vec3(cos(angle) * speed, 5.0 + 2.0 * hash(seed + 2u), sin(angle) * speed);
      pos = vec3(0.0, 0.0, 0.0);
      col = vec4(hsv2rgb(vec3(fract(beat * 0.1 + hash(seed) * 0.2), 0.8, 1.0)), 1.0);
      age = 3.0 + hash(seed + 3u);
    } else {
      // Update simulation step
      vel.y = vel.y - gravity * dt;
      pos = pos + vel * dt;
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

  param point_size : float = 0.015 [0.001, 0.08]

  consumes pos, col, age

  vertex {
    // Project 3D position to clip space
    clip = project(pos);
    point_rate = point_size;
  }

  fragment {
    // Radial soft particle falloff
    let dist = length(uv - vec2(0.5, 0.5)) * 2.0;
    if (dist > 1.0) {
      discard;
    }
    let alpha = (1.0 - dist * dist) * clamp(age, 0.0, 1.0);
    color = vec4(col.rgb * 1.5, alpha);
  }
}
```
