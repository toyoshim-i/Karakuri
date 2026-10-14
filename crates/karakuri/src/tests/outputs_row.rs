//! What the frame is composited at, settled solely by the Program bay selection
//! or the WebXR eye size (ADR-0246, ADR-0394).
//!
//! Output destinations (Program bay preview, external projector, Syphon plugin)
//! are scaled from this single composited frame; their individual sizes do not
//! alter the render resolution.

use super::*;
use crate::bridge::target_render_size;

/// In normal display mode, the Program bay selected resolution decides the frame size.
#[test]
fn the_frame_follows_program_bay_selected_resolution() {
    assert_eq!(
        target_render_size(Some((1280, 720)), None),
        Some((1280, 720))
    );
    assert_eq!(
        target_render_size(Some((1920, 1080)), None),
        Some((1920, 1080))
    );
    assert_eq!(
        target_render_size(Some((1024, 768)), None),
        Some((1024, 768))
    );
    assert_eq!(
        target_render_size(Some((3840, 2160)), None),
        Some((3840, 2160))
    );
}

/// The Program bay preview frame rectangle does not dictate render resolution.
#[test]
fn the_preview_frame_does_not_change_render_size() {
    let selected = Some((1280, 720));
    let preview_rect_size = (466, 262);
    let retina_preview_size = (1800, 1000);

    // The render size remains the operator's selected resolution regardless of preview layout.
    assert_eq!(target_render_size(selected, None), selected);
    assert_ne!(target_render_size(selected, None), Some(preview_rect_size));
    assert_ne!(
        target_render_size(selected, None),
        Some(retina_preview_size)
    );
}

/// External projector windows do not raise or alter the render resolution.
#[test]
fn external_projector_does_not_change_render_size() {
    let selected_1080p = Some((1920, 1080));
    let projector_4k = (3840, 2160);

    // Thermal budget allows rendering at 1080p even when outputting to a 4K display.
    assert_eq!(target_render_size(selected_1080p, None), selected_1080p);
    assert_ne!(target_render_size(selected_1080p, None), Some(projector_4k));
}

/// In WebXR mode, the native eye size decides composite dimensions.
#[test]
fn webxr_eye_size_decides_in_vr_mode() {
    let selected = Some((1920, 1080));
    let eye = Some((960, 1080));
    assert_eq!(target_render_size(selected, eye), eye);
}

/// When no resolution or eye size is provided, returns None.
#[test]
fn no_selected_resolution_or_eye_says_nothing() {
    assert_eq!(target_render_size(None, None), None);
}
