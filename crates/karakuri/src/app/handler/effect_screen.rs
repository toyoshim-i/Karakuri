//! The effect screen: a virtual screen fixed a short way in front of the
//! head, where the WebXR stereo world runs the screen-space effects — the
//! drawn Set's L5s, then the master chain.
//!
//! An L5 only knows its picture's UV. Run on each eye's picture as is, an
//! effect lands at the same UV in both eyes, i.e. at infinity. Instead each
//! eye's picture is resampled onto the screen, the effects run there, and the
//! result is resampled back into the eye; whatever the effects draw is then
//! seen at the screen's depth, while an untouched pixel returns to where it
//! was. The screen's picture is a centre eye with the two eyes' frusta
//! averaged, so neither eye is favoured.
//!
//! Like the scene, the result is reprojected by the XR layer's timewarp: for
//! the frame or so a head turn takes to catch up, the effects move with the
//! world rather than the head.

use karakuri_engine::{DeckSlot, StereoMatrices};

use crate::gfx::{Gfx, StereoTarget};

/// How far in front of the head the effect screen stands, in metres (world
/// units ride the head one to one).
pub(crate) const DISTANCE: f32 = 0.5;

/// Sets both eyes' resamples for this frame's matrices. Writes land at the
/// next submission, so this goes before the first eye's.
pub(crate) fn aim(queue: &wgpu::Queue, target: &StereoTarget, eyes: [&StereoMatrices; 2]) {
    let position = |m: &StereoMatrices| {
        // The view's rotation transposed, applied to minus its translation.
        let v = &m.view;
        let t = v[3];
        [0, 1, 2].map(|i| -(v[i][0] * t[0] + v[i][1] * t[1] + v[i][2] * t[2]))
    };
    let (pl, pr) = (position(eyes[0]), position(eyes[1]));
    let ipd = ((pr[0] - pl[0]).powi(2) + (pr[1] - pl[1]).powi(2) + (pr[2] - pl[2]).powi(2)).sqrt();
    // The centre eye's projection terms: [0][0], [1][1] scale, [2][0],
    // [2][1] the off-centre shift (GL convention, column-major).
    let terms = |m: &StereoMatrices| [m.proj[0][0], m.proj[1][1], m.proj[2][0], m.proj[2][1]];
    let (l, r) = (terms(eyes[0]), terms(eyes[1]));
    let c = [0, 1, 2, 3].map(|i| (l[i] + r[i]) * 0.5);
    // A screen point at centre NDC `n` sits at x = d (n + c20) / c00 in head
    // space; the eye, `side * ipd` along x, sees it at e00 (x - side ipd) / d
    // - e20. Per axis, an affine: e = s n + o.
    let affine = |e: [f32; 4], side: f32| {
        let offset = side * ipd / DISTANCE;
        let s = [e[0] / c[0], e[1] / c[1]];
        let o = [s[0] * c[2] - e[0] * offset - e[2], s[1] * c[3] - e[3]];
        (s, o)
    };
    let raw = [affine(l, -0.5), affine(r, 0.5)];
    // Widen the screen to the union of what the eyes see of it, so that no
    // eye reading back runs off the screen's edge: n = (m - b) / a for the
    // screen's own NDC m, with the union [lo, hi] mapped onto [-1, 1].
    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
    for (s, o) in raw {
        for k in 0..2 {
            lo[k] = lo[k].min((-1.0 - o[k]) / s[k]);
            hi[k] = hi[k].max((1.0 - o[k]) / s[k]);
        }
    }
    let a = [0, 1].map(|k| 2.0 / (hi[k] - lo[k]));
    let b = [0, 1].map(|k| -(hi[k] + lo[k]) / (hi[k] - lo[k]));
    for (eye, (s, o)) in target.eyes.iter().zip(raw) {
        // e = s (m - b) / a + o.
        let s = [0, 1].map(|k| s[k] / a[k]);
        let o = [0, 1].map(|k| o[k] - s[k] * b[k]);
        // Onto the screen: a screen pixel reads the eye at s m + o. Back: an
        // eye pixel reads the screen at (e - o) / s.
        eye.to_screen.set(queue, s, o);
        eye.from_screen.set(
            queue,
            [1.0 / s[0], 1.0 / s[1]],
            [-o[0] / s[0], -o[1] / s[1]],
        );
    }
}

/// Runs the effects for one eye, whose picture is in `target.eyes[at]`, and
/// leaves the result there. Does nothing where there are no effects.
pub(crate) fn record(gfx: &mut Gfx, slot: DeckSlot, encoder: &mut wgpu::CommandEncoder, at: usize) {
    let Some(target) = gfx.stereo_target.as_ref() else {
        return;
    };
    let eye = &target.eyes[at];
    let engine = &mut gfx.engine;
    // The master chain's targets are the Present's size; skip it while that
    // is catching up with a resize.
    let master = engine
        .present
        .chain_entry()
        .filter(|_| engine.present.size() == target.size);
    let set_input = engine.deck.stereo_l5_input(Some(slot));
    match (set_input, master) {
        (None, None) => return,
        (Some(input), _) => target.warp.record(encoder, &eye.to_screen, input),
        (None, Some(entry)) => target.warp.record(encoder, &eye.to_screen, entry),
    }
    if engine.deck.stereo_l5_input(Some(slot)).is_some() {
        let out = master.unwrap_or(&target.screen_view);
        engine.deck.record_stereo_l5s(Some(slot), encoder, out);
    }
    if master.is_some() {
        engine
            .present
            .draw_chain_unheld(encoder, &target.screen_view);
    }
    target.warp.record(encoder, &eye.from_screen, &eye.hdr_view);
}
