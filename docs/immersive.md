# Karakuri Immersive — WebXR & Spatial VJ Output Specification

---

## 1. Vision & Conceptual Framework

Karakuri Immersive extends real-time visual synthesis from traditional 2D flat screens into **true 3D spatial reality**.

Instead of standing in front of a monitor and looking at visual clips, the performer steps **inside the visual instrument itself**:
- The performer stands inside a cyberpunk virtual cockpit.
- Procedural 3D vertices, particle clouds, vector fields, and mesh topologies flow around the performer's body in full 6DoF (six degrees of freedom) stereo space.
- Fragment shaders and master post-processing effects wrap the entire field of view within a celestial dome.
- At the performer's fingertips floats a virtual 30-inch control desk displaying the familiar Karakuri 2D console interface, responsive to VR laser raycasts and pinch interactions.

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
  - Located ~70 cm in front of the performer, tilted up at 35°–45° for comfortable seated or standing interaction.
  - Sized at approximately 30 inches diagonal (~70 cm wide × 40 cm tall), matching a physical DJ mixer / synth workspace.
- **Rendering**:
  - The Karakuri 2D console UI (`egui`) is rendered off-screen into an RGBA8Unorm texture target (e.g. 1920×1080).
  - The texture is mapped onto a planar 3D Quad mesh in the virtual scene.
- **Interaction (UV Raycast Translation)**:
  - WebXR controller ray (or hand-tracking pointer) calculates ray-plane intersection against the Quad mesh.
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

### Tier 3: Celestial Dome Projection (Sky Dome for Fragment & L5 FX)
- **The Problem**:
  - Fragment-only shaders (raymarchers, 2D procedural patterns, screen-space distortions, L5 Master Chain post-effects) have no native 3D vertices and operate in normalized 2D screen UV space.
- **The Solution: Dome-Master Projection**:
  - A large hemisphere or geodesic dome (radius 15–30 m) is rendered in the background, centered on the performer.
  - The fragment/L5 composite is mapped onto the dome's interior surface using polar fisheye / dome-master projection:
    $$u = 0.5 + \frac{\theta}{\pi} \cos(\phi), \quad v = 0.5 + \frac{\theta}{\pi} \sin(\phi)$$
    where $\theta$ is the polar angle from the forward sightline and $\phi$ is the azimuth.
- **Visual Impact**:
  - Becomes an expansive planetarium-style background enclosing the entire horizon and zenith.
  - L5 effects (glitch slices, film grain, strobes, chromatic shifts) pulse across the celestial sphere behind the 3D particles.
  - Depth buffer testing ensures Tier 2 spatial geometry cleanly occludes the celestial dome.

---

## 3. Architecture & Plugin Integration

In alignment with **ADR-0369** (Output plugins sit behind decoupled sink boundaries), Immersive mode is exposed as an output sink:

```
Outputs Bay: [ Monitor ] [ Projector ] [ WebRTC ] [ WebXR / Immersive ]
```

### Decoupled Immersive Sink Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Karakuri Core Engine                            │
│  - Mixer & Decks (L1-L4)                                               │
│  - Master Chain (L5)                                                   │
│  - Console UI (egui)                                                   │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼ (Output Plugin Boundary)
┌────────────────────────────────────────────────────────────────────────┐
│                       Immersive Backend (Trait)                        │
├───────────────────────────────────┬────────────────────────────────────┤
│                                   │                                    │
│   Web Target:                     │   Native Target (Future):          │
│   WebXR Device API                │   OpenXR / SteamVR / visionOS      │
│   (navigator.xr.requestSession)   │   (Direct3D12 / Vulkan / Metal)    │
│                                   │                                    │
└───────────────────────────────────┴────────────────────────────────────┘
```

### Cross-Platform Abstraction: `ImmersiveBackend`
To ensure future compatibility with native VR runtimes (OpenXR on Windows/Linux, visionOS on Apple Vision Pro):

```rust
pub trait ImmersiveBackend: Send + Sync {
    /// Queries whether the platform runtime has an active VR/AR device.
    fn is_available(&self) -> bool;

    /// Requests transition into immersive stereo presentation mode.
    fn request_session(&mut self) -> Result<(), String>;

    /// Polls head and controller poses, returning per-eye view and projection transforms.
    fn update_poses(&mut self) -> Option<StereoPoses>;

    /// Submits the composited stereo frame (Tier 1 HUD + Tier 2 Geometry + Tier 3 Dome).
    fn submit_frame(&mut self, eye_textures: StereoViews) -> Result<(), String>;
}
```

---

## 4. Web Implementation Details (WebXR + WebGPU)

### 4.1 Session Lifecycle
1. User clicks the `[ WebXR ]` pill in the `Outputs` bay.
2. Under the user gesture context, `karakuri-web` calls:
   ```javascript
   navigator.xr.requestSession('immersive-vr', {
       requiredFeatures: ['local-floor'],
       optionalFeatures: ['hand-tracking', 'layers']
   });
   ```
3. Creates an `XRWebGLLayer` or native WebGPU XR binding (`XRWebGPUBinding` where supported).
4. The Web runtime switches into the stereo frame loop (`XRSession.requestAnimationFrame`).

### 4.2 Stereo Eye Rendering Pass
Each frame executes:
1. **Console HUD Pass**: Renders egui into `hud_texture` (1920×1080).
2. **L1 Geometry & L5 Dome Passes**:
   - For each eye (Left / Right):
     - Bind eye-specific view and projection matrices from `XRView`.
     - Render Tier 3 Celestial Dome (sampled from L5/fragment target).
     - Render Tier 2 Spatial Geometries with depth testing (`Depth32Float`).
     - Render Tier 1 HUD Quad (placed at `(0.0, -0.2, -0.7)` with -35° pitch).
3. **Submit**: Present the stereo framebuffer to the WebXR display.

### 4.3 Controller Interaction & Pointer Emulation
- WebXR `XRInputSource` provides gamepad buttons and ray vectors.
- Ray intersects the HUD Quad at local coordinate `(u, v)`:
  - If ray intersects: Emits `CursorMoved { position: (u * w, (1-v) * h) }`.
  - Trigger press: Emits `MouseInput { state: Pressed, button: Primary }`.
  - Thumbstick scroll: Emits `MouseWheel { delta }`.

---

## 5. Phased Implementation Plan

### Phase 1: WebXR Foundation & Virtual 30" HUD (~3 days)
- Add `Outputs::Immersive` toggle pill to console Outputs bay.
- Implement WebXR feature detection (`navigator.xr.isSessionSupported('immersive-vr')`).
- Implement `requestSession('immersive-vr')` activation.
- Render the 30-inch virtual console quad at `(0, -0.2, -0.7)` meters using an off-screen egui texture.
- Map controller raycast intersections to egui mouse clicks and drag events.

### Phase 2: Stereo Camera & 3D Spatial Geometry (~3-4 days)
- Adapt `karakuri-engine` camera parameters to accept external view/projection matrix overrides.
- Connect WebXR stereo eye matrices (`eye.transform.matrix`, `eye.projectionMatrix`) to the rendering pipeline.
- Verify `topology triangles`, `grid`, and `ribbon` procedures render stereoscopically with 6DoF head-tracking parallax.

### Phase 3: Celestial Dome Projection for Fragment/L5 (~2-3 days)
- Implement geodesic background dome mesh (180°–220° field of view).
- Implement polar fisheye texture coordinate mapping in WGSL.
- Composite L5 Master Chain output onto the dome mesh behind Tier 2 geometry.

### Phase 4: Ergonomic Controls & Calibration (~2 days)
- HUD positioning sliders (distance, elevation, tilt angle).
- Passthrough toggle for MR headsets (Quest 3, Vision Pro) using `sessionMode = 'immersive-ar'`.
- Recenter gesture (long-press thumbstick / home button).
