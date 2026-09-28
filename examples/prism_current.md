# Prism Current

A procedural flight through a pleated, twisting polygon tunnel. The GPU generates
6,144 triangles (18,432 vertices) with cyan and vermilion panels, antialiased
emissive seams, travelling light pulses, and distance haze. No textures or models
are required. Motion continues without audio; input energy modulates the pulse
displacement when available.

In the desktop console, select `prism_current` from the preset library. Start the
console from the repository root with `cargo run -p karakuri -- --presets examples`.

For a headless preview, import the preset once into a dedicated store, then render:

```sh
cargo run -p karakuri-cli -- --store /tmp/prism-current-store --take-in examples/prism_current.kset
cargo run -p karakuri-cli -- --store /tmp/prism-current-store --load-set prism_current --render /tmp/prism-current.png --frames 180 --canvas 1280x720
```

The geometry and material are `prism_current.kir` and
`prism_current_surface.kir`; the preset uses the shared `tunnel_eye.kir` camera.
The material's view-dependent sheen and haze are referenced to the tunnel origin.

| Control | Effect |
| --- | --- |
| L1 `speed` | Forward travel rate; zero pauses travel while folds keep moving |
| L1 `fold` | Depth of the eight pleats and smaller surface ripples |
| L1 `twist` | Twist along the tunnel, including reversed winding |
| L1 `surge` | Radial displacement of the travelling pulse; bound to energy |
| L4 `glow` | Brightness of the luminous triangle seams |
| L4 `metal` | Brightness of the solid panels |
| L3 `lens` | Camera field of view |
| L3 `wander` | Camera drift around the tunnel axis |

Use the preset's camera position (`back = 0`) for the interior flight. Geometry
and lighting are functions of time, so seeking does not require a warm-up.
