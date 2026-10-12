---
id: LANE_HELD
aliases: ["lane-held"]
title: Control is held by sequencer lane
summary: Target control or fader is currently captured by an active sequencer pattern lane.
---

# Control is held by sequencer lane

## Behavior

An armed sequencer pattern lane has active playback authority over the target deck control or fader. External MCP updates are refused while sequencer automation is driving the value.

## Remediation

- Mute or disarm the controlling sequencer pattern lane before modifying the parameter.
- Target an unsequenced control or wait until the sequence pattern finishes execution.
