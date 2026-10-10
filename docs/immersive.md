# Karakuri Immersive — WebXR & Spatial VJ Output Specification

---

## 1. Vision & Conceptual Framework

Karakuri Immersive extends real-time visual synthesis from traditional 2D flat screens into **true 3D spatial reality**.

Instead of standing in front of a monitor and looking at visual clips, the performer steps **inside the visual instrument itself**:
- The performer stands inside a cyberpunk virtual cockpit.
- Procedural 3D vertices, particle clouds, vector fields, and mesh topologies flow around the performer's body in full 6DoF (six degrees of freedom) stereo space.
- Fragment shaders and master post-processing effects wrap the entire field of view within a celestial dome.
- At the performer's fingertips floats a virtual 30-inch control desk displaying the familiar Karakuri 2D console interface, responsive to VR laser raycasts and controller interactions.

```
                                ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
                                ┃      Tier 3: Sky Dome          ┃
                                ┃  (Fragment Shaders / L5 Post)  ┃
                                ┃    180°-220° Celestial Canvas  ┃
                                ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
                                                 │
                                ┌────────────────┴───────────────┐
                                │   Tier 2: 3D World Geometry    │
                                │   (L1 Vertices / Ribbons / FX) │
                                │    Direct 6DoF Spatial Field   │
                                └────────────────┬───────────────┘
                                                 │
                                              [ 👤 VJ ]
                                                 │
                                ┌────────────────┴───────────────┐
                                │   Tier 1: 30" Deck Console     │
                                │  (Virtual Floating Quad Mesh)  │
                                │   Interactive egui 2D Surface  │
                                └────────────────────────────────┘
```

---

## 2. Spatial Topology (The Three-Tier Space)

Rather than forcing a flat 2D "fullscreen" concept onto 3D reality, Karakuri Immersive defines a **three-tier spatial topology** where controls, 3D geometry, and planar shaders co-exist harmoniously:

### Tier 1: Personal Deck HUD (30-Inch Virtual Console)
- **Position & Dimensions**:
  - Located ~70 cm in front of the performer, tilted up at 35° for comfortable seated or standing interaction.
  - Sized at approximately 30 inches diagonal (0.72 m wide × 0.405 m tall, 16:9 widescreen), matching a physical DJ mixer / synth workspace.
- **Rendering**:
  - The Karakuri 2D console UI (`egui`) is rendered off-screen into an RGBA8Unorm texture target (e.g. 1920×1080).
  - The texture is mapped onto a planar 3D Quad mesh in the virtual scene.
- **Interaction (UV Raycast Translation)**:
  - WebXR controller ray calculates ray-plane intersection against the Quad mesh.
  - The normalized intersection `(u, v)` coordinates are mapped directly to egui screen-space pixels `(x, y)`:
    $$x = u \times \text{width}, \quad y = (1.0 - v) \times \text{height}$$
  - Standard pointer events (hover, click, drag, scroll) are injected into the existing `winit`/`egui` input queue.
  - **Zero UI Rewrite**: All existing buttons, faders, sliders, menus, and Prompt bay text fields function immediately in VR without modifying a single line of component code.
- **Program Bay Mirror**:
  - The Program bay on this 30" desk continues to draw the 2D broadcast preview frame, ensuring the performer always knows exactly what is being sent to external 2D stream/recording sinks.

### Tier 2: True 3D World Geometry (Direct Spatial Rendering)
- **Geometry Source**:
  - Procedural vertex pipelines: `topology triangles`, `topology grid`, `topology ribbon`, point clouds, and procedural cameras.
- **Camera Coupling**:
  - WebXR supplies per-eye stereo view matrices (`XRView.transform.matrix`) and projection matrices (`XRView.projectionMatrix`) every frame.
  - These matrices are fed directly into Karakuri's 3D shader pipeline (`L4` vertex stage), replacing the fixed monoscopic camera.
- **Visual Impact**:
  - Geometric primitives are rendered directly into the stereo eye buffers.
  - Particles fly past the performer's head; ribbon trails orbit the performer; grids undulate beneath their feet.
  - Complete 6DoF parallax: leaning in, looking around, or walking reveals new angles of the synthesized geometry.

### Tier 3: Built-in VR Projection & Celestial Cosmos
- **The Problem**:
  - Fragment-only shaders (raymarchers, 2D procedural patterns, screen-space distortions, L5 Master Chain post-effects) have no native 3D vertices and operate in normalized 2D screen UV space (`point_coord`).
  - Naively projecting onto viewports creates face-locked visuals, while artificial head-roll can induce motion sickness.
- **The Solution: Room-Space Built-in VR Projection**:
  - The WebGPU shader reconstructs rays in headset room space (`cam.room_fwd`, `cam.room_right`, `cam.room_up`), decoupling virtual projection from head roll and procedural turntable camera rotations.
  - Three distinct projection modes are selectable live from the Master bay:
    1. **`Wall`**: Giant 16:9 flat screen anchored in front of the performer with bounded FOV and zoom control.
    2. **`Dome`**: 240° celestial fulldome mapping zenith $(0.5, 0.5)$ overhead down to $-30^\circ$ below the horizon.
    3. **`Kaleidosky`**: Expansive sky with radial concentric rings (`rings`), azimuthal M-fold rotational symmetry (`facets`), and ping-pong mirroring (`mirror`).
  - **Celestial Cosmos Environment**:
    - Underneath the visual performance, a procedural WebGL2 celestial sphere renders equatorial and elevation grid parallels, meridians, and a 3D procedural starfield.
    - Adjustable in real-time via Master bay parameters: `stars` (brightness), `density` (star count), `grid` (dome line brightness), and `lines` (grid density).
  - Complete shader authoring conventions and mathematical formulas are documented in [ADR-0388](adr/0388-built-in-vr-projection-modes-and-celestial-dome-specification.md) and [`docs/vr-projection.md`](vr-projection.md).

---

## 3. Architecture & Output Sink Integration

In alignment with **ADR-0369** (Output plugins sit behind decoupled sink boundaries), Immersive mode is exposed as an output sink:

```
Outputs Bay: [ Monitor ] [ Projector ] [ WebRTC ] [ WebXR / Immersive ▼ ]
                                                    ├─ VR (Immersive)
                                                    └─ MR (Passthrough)
```

### Decoupled Sink Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Karakuri Core Engine                            │
│  - Mixer & Decks (L1-L4)                                               │
│  - Master Chain (L5)                                                   │
│  - Console UI (egui)                                                   │
│  - Outputs Bay (XrPill: VR / MR Mode Selection)                        │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                     karakuri-web::webxr Subsystem                      │
│  - WebXR Session Lifecycle (immersive-vr / immersive-ar)               │
│  - XRWebGLLayer Stereo Presentation                                    │
│  - 30" Deck HUD Quad Raycast & Pointer Event Translation               │
│  - Locomotion, Turning & 6DoF HUD Grip Manipulation                    │
│  - Timewarp Reprojection & Celestial Cosmos Renderer                   │
└────────────────────────────────────────────────────────────────────────┘
```

- **Outputs Bay Control**: The Outputs bay provides a dedicated WebXR pill with a dropdown menu to select between **VR** (`immersive-vr`) and **MR** (`immersive-ar`).
- **Mixed Reality (MR) Passthrough**: When MR mode is selected, the session requests `immersive-ar`, and the celestial dome background clears with transparent alpha (`vec4(0.0)`), allowing the real-world passthrough environment to show behind the 3D geometry and floating HUD.

---

## 4. Web Implementation Details (WebXR + WebGPU)

### 4.1 Session Lifecycle
1. User clicks the `[ WebXR ]` pill in the `Outputs` bay.
2. Under the user gesture context, `karakuri-web` initiates the session request:
   ```javascript
   navigator.xr.requestSession(sessionMode, {
       requiredFeatures: ['local-floor']
   });
   ```
   where `sessionMode` is `'immersive-vr'` or `'immersive-ar'`.
3. Creates an `XRWebGLLayer` attached to the WebGL2 rendering context.
4. The Web runtime switches into the stereo frame loop (`XRSession.requestAnimationFrame`).
5. Headset and system-level session termination (e.g. Meta button quit, session end) are handled via the `end` event listener, cleanly restoring the 2D canvas display.

### 4.2 Stereo Eye Rendering Pass
Each frame executes:
1. **Console HUD Pass**: Renders egui into the off-screen `hud_texture` (1920×1080).
2. **Stereo Presentation Pass**:
   - For each eye (Left / Right):
     - Bind eye-specific view and projection matrices from `XRView`.
     - Render Tier 3 Celestial Dome Background (reprojected via polar dome shader, or transparent in MR mode).
     - Render Tier 2 Spatial Geometries & Visual Output (WebGPU render target reprojected with timewarp compensation).
     - Render Tier 1 HUD Quad (placed at `(0.0, -0.2, -0.7)` with -35° pitch, with laser hit-test cursor).
3. **Submit**: Present the stereo framebuffer to the WebXR display via `XRWebGLLayer`.

### 4.3 Controller Interaction, Locomotion & Pointer Emulation
- **HUD Quad Interaction (Laser Raycast)**:
  - Controller target ray intersects the HUD Quad at local coordinate `(u, v)`.
  - Laser intersection emits `CursorMoved { position: (u * w, (1-v) * h) }`.
  - Trigger squeeze (Button 0): Emits `MouseInput { state: Pressed/Released, button: Primary }`.
  - Right thumbstick vertical: Emits `MouseWheel { delta }` for console scrolling.
- **Controller Locomotion & Turning**:
  - **Left Thumbstick**: Smooth planar locomotion (forward/backward, strafe left/right) aligned with the player's head facing direction (curved response up to `5.0 m/s`).
  - **Right Thumbstick (Horizontal)**: Smooth yaw rotation of locomotion facing orientation (`2.0 rad/s`).
- **Grip Manipulation of HUD Quad**:
  - Holding the Grip button (Button 1 / Squeeze) allows grabbing, moving, and rotating the virtual console in 3D space with 6DoF controller orientation.
  - While gripping: Thumbstick Y adjusts distance along the controller beam; Thumbstick X scales the HUD quad size (`[0.25, 4.0]`).
- **Recenter**:
  - Recenter is handled natively by the VR/MR headset system menu (e.g. Meta button long-press on Meta Quest), maintaining system standard ergonomics without redundant application-level keybindings.

---

## 5. Implementation Summary

All core capabilities defined in this specification are fully implemented in `crates/karakuri-web/src/webxr/` and integrated with `karakuri-console`:

| Feature Area | Implementation Component | Status |
|---|---|---|
| **Outputs Bay Integration** | `XrPill` in `karakuri-console/src/view/outputs.rs` with VR / MR dropdown | Complete |
| **WebXR Session Management** | `request_webxr_session`, `start_webxr_session`, `immersive-vr` & `immersive-ar` | Complete |
| **30" Deck HUD Quad** | Off-screen egui texture, UV raycasting, pointer event translation (`hud.rs`) | Complete |
| **Stereo 3D Geometry** | Stereo eye view/projection matrices, timewarp reprojection (`world.rs`) | Complete |
| **Built-in VR Projection** | `Wall`, `Dome`, `Kaleidosky` in `fullscreen.rs`, Room-Space ray basis | Complete |
| **Celestial Cosmos** | Procedural starfield, equatorial/elevation parallels, meridians (`world.rs`) | Complete |
| **Locomotion & Turning** | Left stick smooth locomotion (5.0 m/s), Right stick smooth yaw (2.0 rad/s) | Complete |
| **6DoF HUD Manipulation** | Grip grab/reposition/rotate, stick distance, stick scale `[0.25, 4.0]` | Complete |
| **Mixed Reality (MR)** | `immersive-ar` mode, transparent dome clear (`vec4(0.0)`) for passthrough | Complete |
