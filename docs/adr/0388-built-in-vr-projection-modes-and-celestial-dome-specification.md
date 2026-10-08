---
id: 0388
title: Built-in VR projection modes and celestial dome shader specification
status: accepted
date: 2026-10-08
principles: [0084, 0086, 0087, 0090, 0092]
tags: [webxr, vr, projection, dome, kaleidosky, shader, point_coord, cosmos]
---

# Built-in VR projection modes and celestial dome shader specification

## Context

Karakuri's visual synthesis pipeline spans both true 3D spatial vertex pipelines (L1 topologies: points, lines, triangles, grid, ribbon) and 2D fullscreen fragment shaders (L4 procedures without a vertex block, raymarchers, and L5 master post-processing chains).

When transitioning into WebXR immersive reality:
1. True 3D geometry renders with natural stereoscopic 6DoF parallax via headset eye matrices.
2. Fullscreen fragment shaders, however, operate in a 2D screen UV space (`point_coord`). Naively stretching flat 2D shader output across VR head-tracking viewports results in disorienting, face-locked visual artifacts.
3. Conversely, fragment art intended for immersive planetarium domes or ambient visual environments needs to wrap around the observer in world space without artificial rotational roll when the user pitches or turns their head.
4. Furthermore, the virtual cockpit requires an ambient celestial cosmos (equatorial and elevation grid parallels, meridians, and procedural 3D stars) that can be adjusted in real time from the console across all VR projection modes.

## Decision

1. **Room-space ray basis for fullscreen shaders:**
   The camera derivation compute pass calculates `cam.room_fwd`, `cam.room_right`, and `cam.room_up` from headset reference space, decoupling VR visual projection from procedural turntable camera transformations while preserving rock-solid stereoscopic convergence across both eyes.

2. **Three Built-in VR projection modes:**
   - **Mode 1 (`Wall`)**: Giant 16:9 flat virtual screen anchored directly in front of the observer in room space (`room_ray.z < 0.0`), spanning approximately 100° horizontal FOV at default zoom. Rays outside the screen boundary are discarded.
   - **Mode 2 (`Dome`)**: Expanded 240° celestial fulldome spanning from the zenith ($y = 1.0$) down to $-30^\circ$ below the horizon ($y = -0.5$). Polar angle $\theta = \arccos(\text{clamp}(y, -0.5, 1.0))$ maps normalized radius $r = \frac{\theta}{2\pi / 3} \times \text{zoom}$. Out-of-bounds rays ($r > 1.0$ or $y < -0.5$) are discarded.
   - **Mode 3 (`Kaleidosky`)**: Expansive celestial dome with radial concentric ring division (`rings`), azimuthal M-fold rotational symmetry (`facets`), and optional ping-pong mirror reflection (`mirror`).

3. **Coordinate convention for Dome and Kaleidosky shaders:**
   For `Dome` and `Kaleidosky` modes, `in.point_coord` is transformed into normalized dome space $[0.0, 1.0]^2$:
   - Center $(0.5, 0.5)$ corresponds to the **Zenith (straight up)**.
   - Radial distance from center $r = 0.0$ is the zenith; $r \approx 0.75$ is the horizon ($y = 0.0$); $r = 1.0$ reaches $-30^\circ$ below horizon.
   - Raymarchers that explicitly compute view rays (`reads_ray`) bypass this coordinate remap and receive raw room rays.

4. **Master bay Built-in VR parameter controls:**
   The Master bay exposes 8 projection and environment parameters:
   - `zoom`: Scale zoom factor ($[0.2, 4.0]$, default $1.0$).
   - `rings`: Radial N-division for Kaleidosky ($[1.0, 16.0]$, default $4.0$).
   - `facets`: Azimuthal M-division for Kaleidosky ($[0.0, 16.0]$, default $6.0$).
   - `mirror`: Ping-pong reflection vs cyclic repeat for Kaleidosky ($[0.0, 1.0]$, default $1.0$).
   - `stars`: Ambient 3D starfield brightness ($[0.0, 2.0]$, default $1.0$).
   - `density`: Star count / density multiplier ($[0.0, 2.0]$, default $1.0$).
   - `grid`: Celestial dome grid brightness ($[0.0, 2.0]$, default $1.0$).
   - `lines`: Celestial grid line frequency / count ($[0.25, 3.0]$, default $1.0$).

   Parameters inactive in the current mode (e.g. `rings`, `facets`, `mirror` in `Wall` or `Dome` mode) are visually dimmed and styled with muted gray tracks and knob strokes, whereas active parameters render in vibrant theme gradients.

5. **Single-source documentation and MCP publication:**
   The canonical specification and authoring guide for VR projection and celestial dome shaders is maintained at `docs/vr-projection.md` and published directly to Model Context Protocol as `karakuri://vr-projection`.

## Consequences

- Fullscreen shaders, raymarchers, and celestial dome visual procedures can be authored with unambiguous mathematical coordinate contracts.
- Automated agents and human performers can immediately generate or calibrate dome shaders without trial-and-error visual disorientation.
- Performers can adjust environment grid and starfield aesthetics live from the console during spatial performances.
