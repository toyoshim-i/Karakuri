# Cyberpunk and Orbital Dome Demos

Three procedural, fullscreen presets for WebXR's Built-in VR **Dome** mode.
Load a preset from the library, then select **Dome** and **Zoom = 1.0** in the
Master bay. Projection settings are separate from the preset. The desktop
preview shows the dome-master image: centre = zenith, radius 0.75 = horizon.

All three run without audio and bind `pulse` to input energy for gentle light
accents. The skyline, gantries and relay mesh stay in place as you look around.
They are sky projections without positional parallax. Use the built-in
starfield for the background; try **Grid = 0** and **Stars = 0.5**.

## Neon Megacity

[`neon_megacity.kset`](neon_megacity.kset) surrounds the viewer with cyan and
magenta towers, illuminated windows and holographic advertising panels.
Digital rain falls toward the horizon between the buildings. The integer tower
and rain-column counts close the azimuth seam around the back of the viewer.

| L4 control | Effect |
| --- | --- |
| `towers` | Number of buildings, rounded down, from 16 to 64 |
| `rain` | Digital rain brightness; zero removes it |
| `speed` | Rain motion; zero freezes the rain |
| `glow` | Overall brightness |
| `pulse` | Extra advertising panel luminance |

## Orbital Drydock

[`orbital_drydock.kset`](orbital_drydock.kset) places the viewer beneath an
orbital maintenance ring, with twelve docking bays, amber hazard marks and
three cargo shuttles on a circular service lane. An offset blue planet has
procedural cloud cover, directional illumination and a luminous atmosphere.

| L4 control | Effect |
| --- | --- |
| `spin` | Shuttle orbit and cloud drift; zero freezes both |
| `traffic` | Docking signal rate; zero holds the signals |
| `planet` | Planet radius in dome-master space |
| `glow` | Overall brightness |
| `pulse` | Extra docking signal luminance |

## Quantum Relay

[`quantum_relay.kset`](quantum_relay.kset) creates a violet and cyan hexagonal
communications canopy. Light pulses propagate from individual hubs, a scanner
ring sweeps toward the horizon, and a segmented transmitter sits overhead.

| L4 control | Effect |
| --- | --- |
| `mesh` | Hexagon density, from 6 to 20 |
| `flow` | Pulse, scanner and transmitter animation rate; zero freezes them |
| `scan` | Scanner brightness; zero removes the sweep |
| `glow` | Overall brightness |
| `pulse` | Extra scanner and overhead transmitter luminance |

## Desktop Preview

Start the desktop console from the repository root:

```sh
cargo run -p karakuri -- --presets examples
```

For a square dome-master PNG, import the preset once into a dedicated store:

```sh
cargo run -p karakuri-cli -- --store /tmp/cyber-dome-store --take-in examples/neon_megacity.kset
cargo run -p karakuri-cli -- --store /tmp/cyber-dome-store --load-set neon_megacity --render /tmp/neon-megacity.png --frames 360 --canvas 1024x1024
```

Replace `neon_megacity` with `orbital_drydock` or `quantum_relay` to preview
the other presets. Each uses the shared `step_grid.kir` at capacity 256;
the fullscreen renderers read no geometry. No textures or models are required,
and seeking needs no warm-up.
