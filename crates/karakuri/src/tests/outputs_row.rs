//! What the frame is composited at, and what a chip in the Outputs row asks
//! for. Both are decisions rather than device work, so both are here rather
//! than under `mod gpu` — `render_size` is arithmetic over sizes, and an
//! operation is a value.

use super::*;

/// One output, and the frame is that output's size. The ordinary run: the
/// program view is the only sink and the frame follows the Program bay's
/// rectangle.
#[test]
fn the_frame_is_the_one_enabled_outputs_size() {
    assert_eq!(render_size(&[Some((466, 262))]), Some((466, 262)));
}

/// Two outputs, and the frame is the larger — ADR-0247, so that every output is
/// a downscale and none is ever upscaled. Asserted both ways round, because a
/// `max` written the wrong way is right half the time.
#[test]
fn the_frame_is_the_largest_enabled_output() {
    let picture = Some((466, 262));
    let projector = Some((3840, 2160));
    assert_eq!(render_size(&[picture, projector]), projector);
    assert_eq!(render_size(&[projector, picture]), projector);
}

/// An output that is off is not in the maximum. The case an operator makes on
/// purpose: the picture folded away and a projector on, which the manual says
/// leaves the inspector the height.
#[test]
fn an_output_that_is_off_does_not_raise_the_frame() {
    assert_eq!(render_size(&[None, Some((1920, 1080))]), Some((1920, 1080)));
    assert_eq!(render_size(&[Some((640, 360)), None]), Some((640, 360)));
}

/// Every output off says nothing rather than picking a size, which is what lets
/// the caller leave the frame where it is: turning the last sink off stops the
/// publishing and not the instrument (ADR-0171), and reallocating every target
/// to change a picture nobody is looking at is the cost this answer exists to
/// refuse.
#[test]
fn every_output_off_says_nothing() {
    assert_eq!(render_size(&[None, None]), None);
    assert_eq!(render_size(&[]), None);
}

/// The tie goes to the earlier entry, which is the program view — the output an
/// operator is looking at while they decide. `max_by_key` hands back the *last*
/// of several equal maxima, so this is the test that says the fold was written
/// for a reason.
#[test]
fn a_tie_goes_to_the_program_view() {
    let a = Some((1280, 720));
    let b = Some((960, 960));
    // 921600 either way.
    assert_eq!(render_size(&[a, b]), a);
    assert_eq!(render_size(&[b, a]), b);
}

/// The maximum is by pixel count and never componentwise. 1920x1080 beside
/// 1024x1280 must not give 1920x1280, which is a size no output is and which
/// upscales both of them in one axis.
#[test]
fn the_answer_is_always_some_outputs_own_size() {
    let wide = (1920, 1080);
    let tall = (1024, 1280);
    let at = render_size(&[Some(wide), Some(tall)]).expect("two outputs are on");
    assert!(
        at == wide || at == tall,
        "the frame is {at:?}, which is neither {wide:?} nor {tall:?} — a componentwise \
         maximum invents a size no output has and upscales both of them"
    );
    assert_eq!(at, wide);
}
