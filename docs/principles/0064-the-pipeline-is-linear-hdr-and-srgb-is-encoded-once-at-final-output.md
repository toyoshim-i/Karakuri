# The pipeline is linear HDR, and sRGB is encoded once at final output

Every target a procedure draws into is `Rgba16Float`, and every value in it is linear and
unbounded. Values above 1.0 are **expected** rather than tolerated: they are what a bloom
pass and a tone mapper have to work with, and clamping them at any point upstream throws
away the only information those passes read. The transfer to sRGB happens in exactly one
place, [present.rs](../../crates/karakuri-engine/src/present.rs)'s fragment shader, on
values a tone mapper has already brought into `[0, 1]` — which is the only range sRGB
encoding is honest about.

**What it rules out.** An 8-bit intermediate anywhere. It is half the bandwidth and it is
where the design stops working: additive blending of four deck slots needs headroom above
1.0 to blend in, and an `Rgba8Unorm` target has clipped it before the mixer sees it. It
also rules out the second encode, which is the failure this is written to prevent — a pass
that encodes to sRGB "so the intermediate looks right" and hands a display-referred texture
to a downstream pass that treats it as linear. That is invisible in a screenshot and wrong
in every arithmetic operation after it, and nothing in the type system distinguishes the
two textures.

**One call site is the mechanism, not a convention.** `Present` owns both the linear HDR
target every `VideoSource` renders into and the surface the encode writes to, so "encoded
once" is checkable by reading one module rather than by auditing every pass. The encode
itself is the hardware's, from an `Rgba8UnormSrgb` destination format — so no shader
anywhere spells the transfer, and adding a second one would mean adding a second
sRGB-format target, which is a visible thing to review. Tone mapping lives in that same
shader immediately before the encode for the identical reason
([ADR-0037](../adr/0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)).

**Linear means Rec.709 primaries.** Anything folding colour to a scalar says so and uses
them: luminance is `0.2126 R + 0.7152 G + 0.0722 B`, never the mean of three channels,
which would report a saturated blue Set as being as bright as a green one.

**Where it holds.** [present.rs](../../crates/karakuri-engine/src/present.rs) and
[shaders/present.wgsl](../../crates/karakuri-engine/src/shaders/present.wgsl) own the one
encode; [video_source.rs](../../crates/karakuri-engine/src/video_source.rs) states the
target format for every implementor; `render.rs` in
[karakuri-cli](../../crates/karakuri-cli) is the offscreen path, which encodes once for the
same reason and by the same means. Decided in
[ADR-0150](../adr/0150-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once.md).
