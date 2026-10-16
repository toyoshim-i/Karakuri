---
id: 0389
title: Social sharing of Set bundles, remote cloud storage, and clean loop video capture
status: accepted
date: 2026-10-11
principles: [0084, 0086, 0090, 0092]
tags: [web, share, video, social, cloudflare, gist, turnstile, compression, wasm]
---

# Social sharing of Set bundles, remote cloud storage, and clean loop video capture

## Context

Karakuri Web enables real-time procedural shader synthesis entirely in the browser using WebGPU. Sharing visual artifacts with other creators and social platforms (specifically 𝕏 / Twitter) requires both communicating the visual experience (looping video) and transferring the executable state (the `.kbset` bundle containing Set topology and all inlined WGSL shaders) so recipients can load and iterate on the performance.

However, several architectural constraints collided in early web prototypes:
1. **Intrinsic Information Density of WGSL Shaders vs. URL Length Limits:**
   A complete visual Set comprises multiple shader stages (generators, topologies, raymarchers, master post-processing). Bundling all source code via `setfile::bundle` produces 10–50 KB of NDJSON. Even when compressed with raw Deflate (`CompressionStream`) and encoded to Base64URL, URL hashes exceed 4,000 characters for complex sets. Social platforms such as 𝕏 employ strict URL parsing thresholds (`twitter-text` / Netty limits at 4,096 bytes); exceeding this limit causes the URL to be parsed as literal text, overflowing post character limits.
2. **Visual Purity in Media Capture:**
   Recording the main console canvas captures egui UI chrome (bays, faders, knobs, and prompt bar) rather than pure generative art.
3. **Misleading Social Sharing UX:**
   Triggering a video download when clicking "Share on 𝕏" violates user expectations and creates friction during social composition.
4. **Idempotent Store Hydration:**
   When authors open or test their own shared links, or re-open a previously viewed link, the local store already holds the Set ID. Treating existing Set IDs as fatal overwrite errors (`CameFrom::Somebody`) prevented legitimate Set playback.

## Decision

1. **Dual-Tier Sharing Model (Inline Deflate & Remote Cloud Storage):**
   - **Tier 1 (Inline Hash `#set=...`):** For compact Sets under 3,800 characters, the bundle is compressed with raw Deflate and encoded to Base64URL in the URL hash. This requires zero external backend and operates 100% client-side.
   - **Tier 2 (Remote Storage `#url=...`):** When compressed URLs exceed 3,800 characters (or when creators prefer short permalinks), the UI offers two external storage options:
     - **Cloudflare KV (Karakuri Public Storage):** Uploads the `.kbset` bundle to a Cloudflare Worker backed by Workers KV. The endpoint enforces payload limits (128 KB), format verification, content-addressed deduplication, and bot protection via non-interactive Cloudflare Turnstile tokens with production origin validation.
     - **User GitHub Gist:** Allows creators to save their Set as a public Gist using their Personal Access Token (`gist` scope), generating an immutable GitHub raw URL.
   - **Unified Hydration:** Regardless of storage provider, the loader transparently resolves `#url=https://...` (or `#load=...`) alongside `#set=...`, fetching the raw bundle text and hydrating directly onto Deck A.

2. **Pure Visual Offscreen Recording:**
   - Add `recorder_sink: Option<WindowSink>` to `Gfx`, targeting a dedicated offscreen canvas (`#karakuri-recorder-canvas`), dynamically sized to 1080p (1920×1080) or 2K (2560×1440).
   - During 8.0-second video loop capture (480 frames @ 60.0 fps CFR), the engine attaches this surface and overrides internal rendering resolution (`selected_res`) to native canvas resolution, directing tone-mapped render output through `compose` before egui overlays are drawn.
   - The Share modal includes an interactive resolution toggle (1080p FHD @ 16 Mbps vs 2K 1440p @ 28 Mbps) persisted across sessions in `localStorage` (`karakuri_share_video_res`), triggering live canvas resize and recorder sink re-attachment with an asynchronous cancellation token (`captureGeneration`).
   - Tone mapping (`present.wgsl`) applies subtle Interleaved Gradient Noise (IGN) TPDF dithering to protect smooth dark gradients against macroblock collapse during social platform re-encoding.
   - The primary capture pipeline uses the hardware-accelerated **WebCodecs API** (`VideoEncoder` + `VideoFrame`), probing 2K profiles (`avc1.640033` / `avc1.640032`, 28 Mbps) with automatic fallback to 1080p (`avc1.64002A`, 16 Mbps), with 1-second Closed GOP and `prefer-hardware` (Apple VideoToolbox).
   - An in-memory Faststart ISO BMFF MP4 muxer integrates:
     - Non-fragmented box hierarchy (`ftyp` -> `moov` -> `mdat`).
     - QuickTime/VideoToolbox-compatible `colr` (`nclx` Rec.709) and `pasp` (1:1 square pixel) visual sample entry boxes inside `avc1`.
     - 375 distinct, sequential 7-byte silent AAC-LC frames (`mp4a.40.2`, 2,625 bytes total in `mdat`) with strictly monotonic increasing `stco` chunk offsets (ensuring 𝕏's ingestion engine does not drop the audio track due to duplicate offsets).
     - Full 8.0-second duration to guarantee 𝕏 transcodes multi-rendition HLS ladders with the high-resolution gear enabled.
   - Platforms lacking WebCodecs seamlessly fall back to `canvas.captureStream(60)` and `MediaRecorder` with an `AudioContext` silent audio track and 8-second MP4/WebM encoding.

3. **Explicit Two-Step Social Workflow:**
   - Separate media acquisition from social composition:
     - **Step 1:** Explicit `[ ⬇ Step 1: Download Loop Video (8s) ]` button with download confirmation.
     - **Step 2:** Explicit `[ 𝕏 Step 2: Open 𝕏 Post Composer ]` button opening Twitter Web Intent pre-filled with `Generated by #KarakuriVJ` and the short share link.
   - Clear UI guidance instructs the performer to attach the downloaded video in the composer.

4. **Idempotent Set Hydration:**
   - In `import_and_load_bundle`, if `setfile::unbundle` reports that the Set ID already exists in the local store, any missing inlined artifacts (`Record::Src`) are idempotently placed into the content-addressed artifact store, and the existing Set is loaded onto Deck A without raising an error.

## Consequences

- Compact Sets remain 100% serverless, while complex procedural sets share seamlessly via cloud storage without tripping social network character boundaries.
- Captured loop videos showcase pure generative visuals suitable for performance archiving and social media broadcasting.
- Creators can test and re-open their own shared links without receiving store collision errors.
- External storage remains safeguarded against automated abuse and quota exhaustion through Turnstile verification and content-addressing.
