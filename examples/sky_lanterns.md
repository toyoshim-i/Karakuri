# Sky Lanterns

Golden paper lanterns float from every side of the horizon toward the zenith,
under a slow emerald and indigo aurora. Ribbed paper bodies, warm flames and
trailing tassels appear against the built-in starfield. Motion runs without
audio; input energy gently brightens the lanterns.

Load `sky_lanterns.kset` from the preset library. In WebXR immersive VR, select
**Dome** in the Master bay's Built-in VR stage and set **Zoom** to **1.0**.
The centre of the image is directly overhead; the circle at radius 0.75 is the
horizon. Try **Grid = 0**, **Stars = 0.6** for an uncluttered night sky.
Projection settings are selected in the Master bay separately from the preset.
The desktop preview displays the dome-master image. **Kaleidosky** repeats the
lanterns into a patterned canopy.

| L4 control | Effect |
| --- | --- |
| `drift` | Ascent speed; zero holds lantern positions while flames flicker |
| `crowd` | Lantern count, rounded down, from 4 to 8 |
| `veil` | Aurora brightness; zero leaves the lanterns and twilight rim |
| `glow` | Overall brightness |
| `pulse` | Extra lantern luminance, bound to audio energy by the preset |

All visuals are procedural and determined by time. The preset uses the shared
`step_grid.kir` at minimum capacity; the fullscreen renderer reads no geometry.

From the repository root, render a desktop preview:

```sh
cargo run -p karakuri-cli -- --store /tmp/sky-lanterns-store --take-in examples/sky_lanterns.kset
cargo run -p karakuri-cli -- --store /tmp/sky-lanterns-store --load-set sky_lanterns --render /tmp/sky-lanterns.png --frames 180 --canvas 1024x1024
```
