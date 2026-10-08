# Karakuri Built-in VR Projection & Celestial Dome Shader Specification

---

## 1. Overview & Conceptual Architecture

In Karakuri, visual synthesis occurs through both **3D geometric vertex pipelines** (`L1` topologies: `points`, `lines`, `triangles`, `grid`, `ribbon`) and **2D fullscreen fragment pipelines** (`L4` procedures without a vertex block, raymarchers, and `L5` master chain post-effects).

When running in desktop/2D display mode, fullscreen fragment procedures render directly to screen-space NDC quad coordinates $(x \in [-1, 1], y \in [-1, 1])$.

When entering **WebXR Immersive VR**, flat screen coordinates cannot simply be stretched across stereo eye viewports:
- A face-locked flat plane creates unnatural visual strain and motion sickness.
- A virtual projection must maintain **rock-solid stereoscopic convergence** between left and right eyes.
- The projection must be anchored in **headset room space** so that procedural camera movements (such as orbital turntable rotations) do not cause dizzying horizon roll.

To resolve this, Karakuri implements **Built-in VR Projection** directly inside the WebGPU camera shader derivation (`crates/karakuri-codegen/src/l4/fullscreen.rs` and `crates/karakuri-engine/src/shaders/camera.wgsl`).

```
                    ┌──────────────────────────────────────┐
                    │      WebXR HMD Headset Tracking      │
                    │   xr.head, xr.proj, stereo views     │
                    └──────────────────┬───────────────────┘
                                       │
                                       ▼
                    ┌──────────────────────────────────────┐
                    │       Room-Space Ray Basis           │
                    │  cam.room_fwd, room_right, room_up   │
                    └──────────────────┬───────────────────┘
                                       │
         ┌─────────────────────────────┼─────────────────────────────┐
         │                             │                             │
         ▼                             ▼                             ▼
┌──────────────────┐          ┌──────────────────┐          ┌──────────────────┐
│   Mode 1: Wall   │          │   Mode 2: Dome   │          │Mode 3: Kaleidosky│
│ Frontal 16:9     │          │ 240° Celestial   │          │ Radial rings ×   │
│ Virtual Screen   │          │ Fulldome (Sky)   │          │ M-fold symmetry  │
└──────────────────┘          └──────────────────┘          └──────────────────┘
```

---

## 2. The Three Built-in VR Projection Modes

### Mode 1: `Wall` (Frontal Virtual Display Screen)
- **Concept**: Simulates a massive 16:9 cinema/concert screen anchored in front of the observer in virtual room space.
- **Orientation**: Anchored along the forward direction ($-Z$ in room space).
- **Angular Coverage**: At default `zoom = 1.0`, spans approximately $100^\circ$ horizontal FOV.
- **Boundary**: Any ray with $Z \ge -0.01$ (behind observer) or falling outside the $[0, 1]^2$ screen UV rect is cleanly `discard`ed.
- **Ideal Content**: 2D motion graphics, traditional 16:9 generative visuals, video clips, and planar fractal animations.

### Mode 2: `Dome` (Expanded Celestial Fulldome)
- **Concept**: Transforms fullscreen fragment shaders into an expansive planetarium dome overhead, surrounding the observer.
- **Coverage**: From the **Zenith** (straight up, $Y = +1.0$) down to **$-30^\circ$ below the horizon** ($Y = -0.5$), covering a wide $240^\circ$ polar field.
- **Orientation**: Centered on the observer's head position.
- **Boundary**: Polar angle $\theta$ beyond $120^\circ$ ($2\pi / 3$ radians, or $Y < -0.5$) or normalized radius $r > 1.0$ is `discard`ed.
- **Ideal Content**: Sky visualizers, celestial aurora, planetary atmospheres, starry canopies, ambient light baths, and music-reactive horizon sweeps.

### Mode 3: `Kaleidosky` (Kaleidoscopic Celestial Dome)
- **Concept**: Extends the celestial dome with geometric symmetry, repeating and folding the shader pattern radially and azimuthally.
- **Controls**:
  - `rings`: Concentric radial N-division (default $4.0$, range $[1.0, 16.0]$).
  - `facets`: Azimuthal M-fold rotational symmetry (default $6.0$, range $[0.0, 16.0]$).
  - `mirror`: $1.0 = $ ping-pong mirror reflection (seamless tiling), $0.0 = $ cyclic repeat.
  - `zoom`: Scale zoom multiplier ($[0.2, 4.0]$).
- **Ideal Content**: Hypnotic mandala patterns, sacred geometry visualizers, kaleidoscopic light tunnels, and rhythmic fractal bursts.

---

## 3. Coordinate Spaces & Mathematical Contracts

### 3.1 Input UV Coordinates (`in.point_coord`)

In `.kir` and WGSL fragment shaders, fullscreen procedures receive `in.point_coord : vec2<f32>` in the range $[0.0, 1.0]^2$.

#### In `Dome` & `Kaleidosky` Modes:
The engine remaps `in.point_coord` from the 3D room ray:
$$\theta = \arccos(\text{clamp}(y_{\text{room}}, -0.5, 1.0)) \in [0, 2\pi/3]$$
$$r = \frac{\theta}{2\pi / 3} \times \text{zoom} \in [0.0, 1.0]$$
$$a = \operatorname{atan2}(x_{\text{room}}, -z_{\text{room}}) \in [-\pi, \pi]$$
$$\vec{q} = r \begin{pmatrix} \sin(a) \\ \cos(a) \end{pmatrix}$$
$$\text{point\_coord} = \vec{q} \times 0.5 + 0.5$$

#### Key Coordinate Landmarks in Dome Space:
| Landmark | Polar Position | `in.point_coord` |
|---|---|---|
| **Zenith (真上・天頂)** | $r = 0.0$ | `vec2(0.5, 0.5)` (Center of UV space) |
| **Mid-Sky ($+45^\circ$)** | $r \approx 0.375$ | Circle of radius $0.1875$ around center |
| **Astronomical Horizon ($0^\circ$)** | $r \approx 0.75$ | Circle of radius $0.375$ around center |
| **Sub-Horizon ($-30^\circ$)** | $r = 1.0$ | Circle touching the UV edges (`radius = 0.5`) |
| **Ground / Nadir (足元)** | $r > 1.0$ / $Y < -0.5$ | Discarded by engine (transparent) |

> [!IMPORTANT]
> **Zenith is at the center `(0.5, 0.5)`, NOT at `y = 1.0`!**
> When writing a celestial dome shader, patterns centered around the origin $(0, 0)$ in polar coordinates will project directly overhead above the performer's head.

---

### 3.2 Raymarching & Direct 3D Ray Access

If a shader needs to perform volumetric raymarching (e.g. marching through a 3D Signed Distance Field or volumetric clouds), it should read `ray` or compute rays using the camera basis uniforms:

```wgsl
// NDC coordinate reconstructed from point_coord:
let _ndc = vec2<f32>(in.point_coord.x * 2.0 - 1.0, 1.0 - in.point_coord.y * 2.0);

// World-space view ray (tracks head orientation and orbital camera):
let world_ray = normalize(cam.fwd + cam.right * _ndc.x + cam.up * _ndc.y);

// Headset room-space ray (anchored to virtual cockpit, immune to turntable camera spin):
let room_ray = normalize(cam.room_fwd + cam.room_right * _ndc.x + cam.room_up * _ndc.y);
```

When a procedure block contains references to `ray` (`reads_ray`), the Built-in VR projection pass preserves raw coordinates without applying planar remapping, allowing the raymarcher to cast rays directly into 3D space.

---

## 4. Shader Authoring Guide & Code Examples

### 4.1 Converting `in.point_coord` to Polar Coordinates
This helper converts `in.point_coord` into radial distance $r \in [0.0, 1.0]$ and angle $\phi \in [-\pi, \pi]$:

```kir
// Normalize to [-1.0, 1.0] relative to Zenith center
let p = (in.point_coord - vec2(0.5, 0.5)) * 2.0;
let r = length(p);          // 0.0 at zenith, ~0.75 at horizon, 1.0 at -30 deg
let phi = atan2(p.y, p.x);  // Azimuthal angle around the zenith
```

### 4.2 Handling Horizon Edge Falloff
To prevent a harsh cutoff at the dome boundary, apply a soft smoothstep falloff near $r = 1.0$:

```kir
let horizon_mask = 1.0 - smoothstep(0.85, 1.0, r);
color = vec4(color.rgb * horizon_mask, color.a * horizon_mask);
```

---

### 4.3 Practical Examples

#### Example 1: Celestial Aurora Canopy (`L4` Fullscreen)
Creates shimmering curtains of northern lights swirling around the zenith:

```kir
proc celestial_aurora {
  kind L4
  blend additive

  param wave_speed : float = 0.8 [0.1, 3.0]
  param ribbon_freq : float = 6.0 [1.0, 16.0]
  param intensity : float = 1.2 [0.0, 3.0]

  fragment {
    // 1. Transform to polar coordinates around Zenith
    let p = (in.point_coord - vec2(0.5, 0.5)) * 2.0;
    let r = length(p);
    let phi = atan2(p.y, p.x);

    // 2. Multi-layered swirling spiral waves
    let t = time * wave_speed;
    let wave1 = sin(phi * ribbon_freq + r * 8.0 - t * 2.0);
    let wave2 = cos(phi * (ribbon_freq * 0.5) - r * 12.0 + t * 1.5);
    let curtain = pow(0.5 + 0.5 * (wave1 * 0.6 + wave2 * 0.4), 3.0);

    // 3. Elevation modulation: brighter near mid-sky, soft at zenith & horizon
    let elevation_weight = smoothstep(0.05, 0.3, r) * (1.0 - smoothstep(0.8, 1.0, r));

    // 4. Music-reactive spectral color gradient
    let hue = fract(0.35 + sin(t * 0.2 + r * 2.0) * 0.15 + audio.y * 0.2);
    let rgb = hsv2rgb(vec3(hue, 0.85, 1.0));

    let final_lum = curtain * elevation_weight * intensity * (1.0 + audio.z * 1.5);
    color = vec4(rgb * final_lum, final_lum);
  }
}
```

#### Example 2: Planetarium Cosmic Radial Clock (`L4` Fullscreen)
Draws geometric concentric orbital rings and rotating pulse beams across the sky:

```kir
proc planetarium_rings {
  kind L4
  blend additive

  param ring_count : float = 8.0 [2.0, 16.0]
  param beam_width : float = 0.04 [0.01, 0.2]

  fragment {
    let p = (in.point_coord - vec2(0.5, 0.5)) * 2.0;
    let r = length(p);
    let phi = atan2(p.y, p.x);

    // Concentric elevation rings
    let ring_phase = fract(r * ring_count - beat * 0.25);
    let ring_line = 1.0 - smoothstep(0.0, 0.08, abs(ring_phase - 0.5));

    // Rotating radial scanner beam
    let beam_angle = mod(phi - beat * 0.5, 6.2831853);
    let beam = smoothstep(beam_width, 0.0, beam_angle);

    // Horizon soft falloff
    let mask = 1.0 - smoothstep(0.85, 1.0, r);

    let lum = (ring_line * 0.4 + beam * 1.2) * mask;
    let col = mix(vec3(0.1, 0.6, 1.0), vec3(0.9, 0.3, 1.0), r);
    color = vec4(col * lum, lum);
  }
}
```

---

## 5. Master Bay Controls & Environment Parameters

The Master bay Built-in VR stage exposes 8 live sliders:

| Key | Label | Range | Default | Active Modes | Description |
|---|---|---|---|---|---|
| `Zoom` | `zoom` | $[0.2, 4.0]$ | `1.00` | All (`Wall`, `Dome`, `Kaleidosky`) | Field of view / projection scale factor |
| `Rings` | `rings` | $[1.0, 16.0]$ | `4.00` | `Kaleidosky` only | Concentric radial ring division count |
| `Facets` | `facets` | $[0.0, 16.0]$ | `6.00` | `Kaleidosky` only | Azimuthal M-fold rotational symmetry |
| `Mirror` | `mirror` | $[0.0, 1.0]$ | `1.00` | `Kaleidosky` only | Ping-pong reflection ($1.0$) vs cyclic ($0.0$) |
| `Stars` | `stars` | $[0.0, 2.0]$ | `1.00` | All | Ambient 3D starfield brightness ($0.0 = $ off) |
| `Density` | `density` | $[0.0, 2.0]$ | `1.00` | All | 3D star count / distribution density |
| `Grid` | `grid` | $[0.0, 2.0]$ | `1.00` | All | Celestial dome grid line brightness ($0.0 = $ off) |
| `Lines` | `lines` | $[0.25, 3.0]$ | `1.00` | All | Celestial grid parallel & meridian frequency |

### Visual Feedback Rule
- **Active Parameters**: Render with high-contrast text and vibrant theme fader tracks (gradient from mint to lavender).
- **Inactive Parameters** (e.g. `rings`, `facets`, `mirror` while in `Wall` or `Dome` mode): Render with dimmed text and muted grayscale fader tracks/knobs (`pal.well`, `pal.line`, `pal.panel`) to immediately inform the operator that adjusting them will have no effect in the current projection mode.

---

## 6. Background Celestial Environment Composite Model

In addition to user-authored shaders, Karakuri's WebGL2 XR world renderer (`crates/karakuri-web/src/webxr/world.rs`) renders an ambient cosmic sphere in world space:

1. **Atmospheric Gradient**: Soft vertical color blend from zenith dark navy `(0.010, 0.014, 0.026)` to horizon indigo `(0.016, 0.020, 0.035)`.
2. **Equatorial Horizon & Parallels**: Spaced by $\Delta \text{lat} = \frac{\pi}{6 \times \text{lines}}$ (default every $30^\circ$).
3. **Meridians**: Spaced by $\Delta \text{az} = \frac{\pi}{6 \times \text{lines}}$ (default 12 meridians around the horizon).
4. **Procedural 3D Starfield**: Dual-layer pseudo-random hash field fixed to the celestial sphere with twinkle animations.

Visual procedures composited on top of this background use **additive luminance blending** (`scene_color += tex.rgb`), ensuring that visual procedural performances illuminate and interact with the cosmic night sky.
