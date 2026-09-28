---
id: 0374
title: The built-in camera expands to six placement parameters and procedural L3 camera procedures
status: accepted
date: 2026-09-27
supersedes: [0318]
superseded_by: []
principles: [0084, 0086, 0090, 0092]
tags: [engine, camera, l3, parameters, examples, m9]
---

# The built-in camera expands to six placement parameters and procedural L3 camera procedures

## Context

[ADR-0318](0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md) resolved the long-standing
question of the built-in camera by exposing three fields of `Orbit` as inspectable parameter rows:
`radius`, `speed`, and `height`. Under ADR-0318, the target was permanently locked to origin `(0, 0, 0)`,
and camera roll (Dutch angle banking) was completely unsupported.

While three parameters provided basic circular turntable viewing, live visual performance demands expressive,
cinematic camera choreographies:
- Focusing on tall or hovering visual elements required a non-zero vertical look-at target (`target_y`).
- Banking during aggressive musical transitions or beat drops required rotation around the line of sight (`roll`).
- Rhythmic vertical breathing (`bob`) and distance pumping (`dolly`) required harmonic oscillation over time.
- Static manual framing required azimuth turntable orientation (`yaw`) and lens field-of-view adjustment (`fov_y`).

Under ADR-0318, performing even simple camera variations—such as a swinging pendulum, a banking fly-by, or a Dutch
angle—forced operators to author bespoke L3 compute shaders from scratch. This high authoring barrier degraded
live improvisation and hindered rapid setfile preparation.

## Decision

**1. Supersede ADR-0318 and Expand Orbit Placement Parameters to Nine:**
- Expand `Orbit::PLACEMENT` in `karakuri-engine/src/camera.rs` by adding six parameters to the original three:
  ```rust
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
  ```
- **`fov_y`** (`[0.2, 2.5]`, default `π/3`): Vertical lens field of view in radians.
- **`yaw`** (`[-π, π]`, default `0.0`): Turntable azimuth angle offset in radians.
- **`target_y`** (`[-20.0, 20.0]`, default `0.0`): Vertical look-at elevation offset (`target = [0.0, target_y, 0.0]`).
- **`roll`** (`[-π, π]`, default `0.0`): Roll rotation angle around the forward look-at axis (Dutch angle).
- **`bob`** (`[0.0, 10.0]`, default `0.0`): Vertical harmonic oscillation amplitude, driven by orbital frequency.
- **`dolly`** (`[-20.0, 20.0]`, default `0.0`): Distance modulation offset, clamped to prevent negative radius.

**2. Rodrigues' Rotation Formula for Camera Roll:**
- Compute orthonormal basis vectors from eye to target:
  ```rust
  let fwd = normalize(sub(target, eye));
  let right = if fwd[0].abs() < 1e-4 && fwd[2].abs() < 1e-4 {
      [1.0, 0.0, 0.0]
  } else {
      normalize(cross(fwd, [0.0, 1.0, 0.0]))
  };
  let base_up = cross(right, fwd);
  let up = if self.roll.abs() > 1e-5 {
      add(scale(base_up, self.roll.cos()), scale(right, self.roll.sin()))
  } else {
      base_up
  };
  ```
- Both host CPU derivation and the GPU compute shader (`camera.wgsl`) execute bit-identical Rodrigues math,
  satisfying principle [P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md).

**3. Standard Procedural L3 Camera Procedures and Sets:**
- Ship four production-ready, expressive L3 procedural camera procedures in `examples/` with accompanying `.kset` configurations:
  1. `centroid_tracker.kir`: Dynamically tracks moving geometry, centering focal points with adaptive height.
  2. `lissajous_orbit.kir`: Navigates a 3D Lissajous / figure-8 trajectory (harmonic ratios 1 : 2 : 3) with dynamic banking roll.
  3. `pendulum_swing.kir`: Simulates harmonic pendulum oscillation, sweeping back and forth across the focal zone.
  4. `fly_between.kir`: Waypoint traversal smoothly interpolating camera eye and look-at positions.

## Alternatives rejected

- **Maintaining ADR-0318's three-parameter ceiling**: Forcing operators to write custom L3 procedural shaders
  for standard Dutch angles, target offsets, or field-of-view changes created unacceptable friction during live sets.
- **Exposing raw 4x4 transform matrices**: Arbitrary matrix writes bypass declared parameter bounds
  ([P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md)), risk singular or non-invertible frustums
  ([P-0084](../principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)), and prevent
  meaningful slider and MIDI knob mapping.
- **Quaternions for camera orientation**: While numerically general, unit quaternions present non-intuitive 4D fader
  mappings to human operators; spherical coordinates (`yaw`, `height`, `radius`) combined with Rodrigues `roll` map
  linearly to physical stage intuitions.
