//! **The Master bay's out: the console's thirteenth control, and the first
//! with a handle that is not in the Mixer bay.**
//!
//! Seven things, and the first two are why this is its own file rather than a
//! few more assertions in `fader.rs`:
//!
//! 1. Where the row is, measured off the bay's own rectangle — the mock's
//!    `.master-body` padding under the bay head, and `.master-row`'s three
//!    items across it.
//! 2. **That the knob clears every boundary's grab at both ends of its
//!    travel**, which is `tests/outputs.rs`'s arithmetic over a control that
//!    *moves*: a knob at 0.00 and a knob at 1.00 are two rectangles, and the
//!    one nearest a boundary is not the same one at both.
//! 3. That the figure's box holds every reading and does not move, which is
//!    what stops the knob walking away from the hand dragging it — and that
//!    the knob itself does move, which is the only thing here that should.
//! 4. **That the knob is the target and the track is not** — `Mixer::grab`'s
//!    rule, and deliberately not the exposure track's.
//! 5. What a drag asks for: `SetMasterOut`, exactly at both ends, and naming
//!    no deck.
//! 6. That a console with no engine behind it draws no row and claims no
//!    press.
//! 7. **The route a window loop actually takes** — `claim`, then the
//!    derivation that drew the control, then the operation, then the release.
//!
//! None of it needs a window, a device or a disk. It does need `egui`'s fonts,
//! because the figure's box is as wide as the widest reading it can hold — see
//! `common::drawn_once` — and a level, because a console with no engine behind
//! it draws no row at all.

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Dragged, Knob, Panel, Released, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{master, Chain, Fx, MasterRow, View};
use karakuri_layout::{Point, Rect};
use karakuri_operation::{Cut, Feedback, Operation};

/// **The mock's own level**: `out 1.00`, which is what
/// `docs/manual/console.html`'s Master bay draws and what
/// `karakuri_engine::deck::Deck` comes up at.
const MOCK: f32 = 1.0;

/// A view with an engine behind it and that level in front of it — what
/// `View::draw` paints from and what `claim` hit-tests, one value.
fn view(out: f32) -> View {
    let mut view = View::new(Room::Day);
    view.master_out = Some(out);
    view.master_chain = Some(MOCK_CHAIN);
    view
}

/// **The mock's own chain**: `feedback 0.34 · mix`, `bloom 0.60`,
/// `rgb shift 0.00`, which is what `docs/manual/console.html` draws under the
/// out row. The rgb shift is the one at zero on purpose — it is the row the
/// page draws dim, and dim here means *this pass is not recorded at all*.
const MOCK_CHAIN: Chain = Chain {
    feedback: 0.34,
    cut: Cut::Mix,
    bloom: 0.60,
    rgb_shift: 0.0,
};

/// A panel at a viewport, solved, with a context that has drawn once — the
/// pair every test here starts from, and `look.rs`'s own opening.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// The laid-out row at that level, on a solved console.
fn row(panel: &Panel, ctx: &egui::Context, out: f32) -> MasterRow {
    master(ctx, panel.layout(), Some(out), Some(MOCK_CHAIN))
        .expect("the Master bay draws its out row")
}

/// A `karakuri_layout` point, from `egui`'s.
fn at(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

// ---------------------------------------------------------------------------
// Where the control is
// ---------------------------------------------------------------------------

/// **The row is the bay's own geometry**: `.master-body`'s padding under the
/// bay head, and `.master-row`'s three items across it with the fader taking
/// what is left.
///
/// Every number here is one of `room::size`'s, so this fails if the mock's
/// padding or gap is transcribed differently and not if a font is.
#[test]
fn the_out_row_is_the_bays_own_geometry() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        let (panel, ctx) = console(viewport);
        let bay = rect_of(panel.layout(), "master");
        let it = row(&panel, &ctx, MOCK);

        assert!(
            near(it.label.min.x, bay.x + size::MASTER_PAD_X),
            "the `out` starts at {} and `.master-body`'s padding is {} in from {}",
            it.label.min.x,
            size::MASTER_PAD_X,
            bay.x
        );
        assert!(
            near(it.label.min.y, bay.y + size::HEAD_H + size::MASTER_PAD_TOP),
            "the row starts at {} — the bay head is {} and `.master-body` pads {} under it",
            it.label.min.y,
            size::HEAD_H,
            size::MASTER_PAD_TOP
        );
        assert!(
            near(it.label.height(), size::MASTER_ROW_H),
            "the row is {} tall and `.master-row` is one line of type at {}",
            it.label.height(),
            size::MASTER_ROW_H
        );
        // The figure is last and ends at the padding, so the track is what is
        // left between the two — which is `.master-row`'s `flex: 1`.
        assert!(
            near(it.value.max.x, bay.x + bay.w - size::MASTER_PAD_X),
            "the figure ends at {} and the bay's right padding is at {}",
            it.value.max.x,
            bay.x + bay.w - size::MASTER_PAD_X
        );
        assert!(
            near(it.fader.track.min.x, it.label.max.x + size::MASTER_GAP)
                && near(it.fader.track.max.x, it.value.min.x - size::MASTER_GAP),
            "the track runs {}..{} and `.master-row`'s gap of {} puts it at {}..{}",
            it.fader.track.min.x,
            it.fader.track.max.x,
            size::MASTER_GAP,
            it.label.max.x + size::MASTER_GAP,
            it.value.min.x - size::MASTER_GAP
        );
        assert!(
            near(it.fader.track.height(), size::FADER_H),
            "the track is {} tall and `.fader` is {}",
            it.fader.track.height(),
            size::FADER_H
        );
        assert!(
            near(it.fader.track.center().y, it.label.center().y),
            "`.master-row` is `align-items: center` and the track is off the line's middle"
        );
    }
}

/// **The figure's box holds every reading this control can ask for, and it
/// does not move.**
///
/// The mock gives the fader `flex: 1` and puts the figure after it, so the
/// track's far end is wherever the figure begins — and a box sized to what it
/// says would resize *while the value is being dragged*, taking the knob under
/// the hand with it. So the box is as wide as the widest reading the control
/// can ask for, which is what these two assertions are between them: it is
/// never narrower than the reading in it, and it is the same box at every one.
///
/// **The width is measured from `egui` here rather than asked of the crate**,
/// so a box too small is two numbers disagreeing rather than one derivation
/// agreeing with itself — `tests/fader.rs`'s rule about a travel.
#[test]
fn the_figures_box_holds_every_reading_and_does_not_move() {
    let (panel, ctx) = console(PLAUSIBLE);
    let first = row(&panel, &ctx, 0.0);
    for out in [0.0, 0.08, 0.25, 0.5, 0.777, 0.99, MOCK] {
        let it = row(&panel, &ctx, out);
        let reading = text_width(&ctx, &format!("{out:.2}"));
        assert!(
            it.value.width() >= reading,
            "the figure's box is {} wide and `{out:.2}` lays out at {reading} — the reading is \
             painted outside the box the row reserved for it",
            it.value.width()
        );
        assert_eq!(
            it.value, first.value,
            "the figure's box is a different box at {out}, so the track moves as it is dragged"
        );
        assert_eq!(
            it.fader.track, first.fader.track,
            "the track moved at {out}, so setting the level means chasing a control that walked \
             away"
        );
        assert_eq!(it.label, first.label);
    }
}

/// **The knob follows the level**, which is the one thing in the row that is
/// meant to move — it sits on the fill's moving edge, and that is the whole of
/// what a fader draws.
#[test]
fn the_knob_and_the_fill_follow_the_level() {
    let (panel, ctx) = console(PLAUSIBLE);
    let track = row(&panel, &ctx, 0.0).fader.track;
    let mut last = f32::NEG_INFINITY;
    for out in [0.0, 0.25, 0.5, 0.75, MOCK] {
        let it = row(&panel, &ctx, out);
        assert!(
            near(it.fader.knob.center().x, track.min.x + track.width() * out),
            "the knob is at {} at a level of {out} and the fill's edge on a track of {} is at {}",
            it.fader.knob.center().x,
            track.width(),
            track.min.x + track.width() * out
        );
        assert!(
            near(it.fader.fill.max.x, it.fader.knob.center().x),
            "the knob is not centred on the fill's moving edge, so the fader has two ideas of \
             what the level is"
        );
        assert!(
            it.fader.knob.center().x > last,
            "the knob did not move between {last} and a level of {out}"
        );
        last = it.fader.knob.center().x;
    }
}

/// **What a run of text lays out to**, asked of the same fonts the row was
/// measured against — the one thing in this file that is `egui`'s answer and
/// not the console's.
fn text_width(ctx: &egui::Context, text: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            egui::FontId::new(size::BASE, egui::FontFamily::Proportional),
            egui::Color32::PLACEHOLDER,
        )
        .size()
        .x
    })
}

// ---------------------------------------------------------------------------
// The claim rule
// ---------------------------------------------------------------------------

/// **The knob clears every boundary's grab, at both ends of its travel.**
///
/// Measured here and never inherited from another control, which is `input`'s
/// rule about every one of the thirteen. Above the knob the clearance is a
/// sum rather than a centring: the bay head is painted over the top of the
/// region, `.master-body` pads under it, and the 11-tall knob is centred in a
/// row of 16.5 — 27 + 8 + 2.75 = **37.75**. Down the sides it is the body's
/// own padding plus the `out` and the figure either side of the track, less
/// the half-knob that overhangs each end of it. **Below** is whatever is left
/// of the bay under the row, and that one is measured rather than computed:
/// this bay is `flex: 1` in its column.
///
/// **Both ends, because this control moves.** A knob at 0.00 and a knob at
/// 1.00 are two rectangles at opposite ends of the track, and the boundary
/// nearest one of them is not the one nearest the other.
///
/// **So it fails if the row moves, if the bay's padding shrinks, or if `GRAB`
/// widens past 37.75** — and the last is the point: the fix then is to change
/// the rule in `input`, deliberately.
#[test]
fn the_knob_clears_every_boundarys_grab() {
    for viewport in [SMALLEST, PLAUSIBLE] {
        for out in [0.0, MOCK] {
            let (mut panel, ctx) = console(viewport);
            let it = row(&panel, &ctx, out);
            let bay = rect_of(panel.layout(), "master");
            let view = view(out);
            let knob = it.fader.knob;

            let above = knob.min.y - bay.y;
            assert!(
                near(above, size::HEAD_H + size::MASTER_PAD_TOP + 2.75),
                "the knob has {above} of bay above it, and 27 of head plus 8 of padding plus \
                 the 2.75 an 11px knob is inset in a row of 16.5 is 37.75"
            );
            let below = bay.y + bay.h - knob.max.y;
            for (what, gap) in [("above", above), ("below", below)] {
                assert!(
                    gap > GRAB,
                    "the knob has {gap} of bay {what} it and a boundary grabs {GRAB} — the \
                     control is inside a boundary's grab, and `input`'s rule is what has to \
                     change"
                );
            }
            for (what, gap) in [
                ("left", knob.min.x - bay.x),
                ("right", bay.x + bay.w - knob.max.x),
            ] {
                assert!(
                    gap > GRAB,
                    "the knob at {out} has {gap} of bay to its {what} and a boundary grabs \
                     {GRAB} — the `.master-body` padding is what holds it off, and it no \
                     longer does"
                );
            }

            for probe in [
                knob.left_top(),
                knob.right_top(),
                knob.left_bottom(),
                knob.right_bottom(),
                knob.center(),
            ] {
                assert!(
                    !matches!(
                        panel.layout().hit(at(probe), GRAB),
                        karakuri_layout::Hit::Divider { .. }
                    ),
                    "a boundary grabs {probe:?}, which is on the master out's knob"
                );
                assert_eq!(
                    claim(&mut panel, &ctx, &view, at(probe)),
                    Claim::Panel,
                    "the panel does not get a press at {probe:?}, which is on the master out"
                );
            }

            // The band under the bay is still the boundary's, which is what
            // says the clearance is a clearance and not the grab having gone
            // missing.
            assert!(
                matches!(
                    panel
                        .layout()
                        .hit(Point::new(knob.center().x, bay.y + bay.h), GRAB),
                    karakuri_layout::Hit::Divider { .. }
                ),
                "the boundary under the Master bay is not grabbable at all, so this test is no \
                 longer measuring the clearance it was written for"
            );
        }
    }
}

/// **The knob is the target and the track is not**, which is `Mixer::grab`'s
/// rule and is deliberately not the exposure track's: this fader has a handle
/// drawn on it, and a handle that jumped to the pointer would be a lie about
/// what a handle is.
#[test]
fn the_knob_is_grabbed_and_the_track_is_not() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, 0.5);
    let view = view(0.5);

    let took = it
        .grab(at(it.fader.knob.center()))
        .expect("the master out's knob");
    assert_eq!(
        took.knob(),
        Knob::Out,
        "the master out named a deck's fader"
    );

    for (what, p) in [
        ("the track's left end", it.fader.track.left_center()),
        ("the track's right end", it.fader.track.right_center()),
        ("the `out` label", it.label.center()),
        ("the figure", it.value.center()),
    ] {
        assert!(
            it.grab(at(p)).is_none(),
            "{what} took the master out in hand, and a press off the knob must not move the mix"
        );
        assert!(!it.owns(at(p)), "{what} is claimed and acts on nothing");
        assert_eq!(
            claim(&mut panel, &ctx, &view, at(p)),
            Claim::Egui,
            "the panel claims {what}, which it would then throw away"
        );
    }
}

// ---------------------------------------------------------------------------
// What a drag asks for
// ---------------------------------------------------------------------------

/// **Both ends of the drag are exact, and neither names a deck.**
///
/// `SetMasterOut` is the one operation in the mixing group that carries no
/// slot: it is a level on the whole fold rather than on a member of it
/// (ADR-0224). The ends are exact for `Grab::value`'s reason — the
/// subtraction is zero at one end and the division is `travel / travel` at
/// the other.
#[test]
fn a_drag_asks_for_a_master_out_at_both_ends() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, 0.5);
    let grab = it
        .grab(at(it.fader.knob.center()))
        .expect("the master out's knob");

    for (end, want) in [(it.fader.track.min.x, 0.0), (it.fader.track.max.x, 1.0_f32)] {
        panel.grab(at(it.fader.knob.center()), grab.clone());
        let to = Point::new(end, it.fader.knob.center().y);
        assert_eq!(
            panel.moved(to),
            Some(Dragged::Fader(Operation::SetMasterOut { out: want })),
            "the master out dragged to {end} asked for something other than {want}"
        );
        assert_eq!(
            panel.released(None),
            Some(Released::Let { knob: Knob::Out }),
            "the release names a deck's fader rather than the master out"
        );
    }
}

/// **A console with no engine behind it draws no row and claims no press.**
///
/// `View::master_out` is `None` until whoever owns a deck writes it, which is
/// every other test in this crate — and the bay then draws its card and its
/// head, exactly as the Mixer bay does with no strips.
#[test]
fn no_level_behind_the_console_is_no_row_at_all() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    assert!(
        master(&ctx, panel.layout(), None, Some(MOCK_CHAIN)).is_none(),
        "the Master bay drew an out row for a console with no engine behind it"
    );
    assert_eq!(
        claim(
            &mut panel,
            &ctx,
            &View::new(Room::Day),
            at(it.fader.knob.center())
        ),
        Claim::Egui,
        "the panel claims the press where a knob would be if there were a level to draw one at"
    );
}

/// **The route a window loop actually takes**, end to end: `claim` says the
/// panel's, the derivation that drew the control says what is in hand, the
/// drag says what it asks for, and the release says what was let go.
///
/// The same three steps `tests/look.rs` walks over the two controls at the end
/// of the transport row, and it is what says the thirteenth control is wired
/// the way the other twelve are rather than merely drawn.
#[test]
fn the_route_a_window_takes_is_claim_then_derivation_then_operation() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let view = view(MOCK);
    let it = row(&panel, &ctx, MOCK);
    let press = at(it.fader.knob.center());

    assert_eq!(claim(&mut panel, &ctx, &view, press), Claim::Panel);
    let grab = master(&ctx, panel.layout(), view.master_out, view.master_chain)
        .and_then(|row| row.grab(press))
        .expect("the derivation that drew the knob says what a press on it takes hold of");
    panel.grab(press, grab);
    // Rule 1: a drag in hand keeps the claim wherever the pointer wanders to,
    // and the pointer on this control wanders across the bay's whole width.
    let to = Point::new(it.fader.track.min.x, press.y);
    assert_eq!(claim(&mut panel, &ctx, &view, to), Claim::Panel);
    assert_eq!(
        panel.moved(to),
        Some(Dragged::Fader(Operation::SetMasterOut { out: 0.0 }))
    );
    assert_eq!(
        panel.released(None),
        Some(Released::Let { knob: Knob::Out })
    );
}

// ---------------------------------------------------------------------------
// The master chain's three rows
// ---------------------------------------------------------------------------

/// **The three rows are drawn, in the chain's order**, and each reads the
/// value the chain handed in — which is the mock's, row for row.
#[test]
fn the_bay_draws_the_chains_three_passes_in_order() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    let drawn: Vec<Fx> = it.fx.iter().flatten().map(|row| row.fx).collect();
    assert_eq!(
        drawn,
        Fx::ALL.to_vec(),
        "the Master bay drew the chain in an order the engine does not run it in"
    );
    let amounts: Vec<f32> = it.fx.iter().flatten().map(|row| row.amount).collect();
    assert_eq!(amounts, vec![0.34, 0.60, 0.0]);
    // **Under the out row and not over it**, which is the chain's own
    // direction: the level enters at the top and the passes are downstream of
    // it (ADR-0224).
    let first = it.fx[0].expect("the feedback row").well;
    assert!(
        first.min.y >= it.fader.track.max.y,
        "an effect row was drawn over the out fader"
    );
}

/// **The dot and the dim say whether a pass is recorded at all.** An amount of
/// zero is not a pass multiplying by nothing — no pass is recorded — so the
/// row that reads 0.00 is the row that is out.
#[test]
fn a_pass_at_zero_is_the_row_that_is_out() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    let runs: Vec<bool> = it.fx.iter().flatten().map(|row| row.runs).collect();
    assert_eq!(
        runs,
        vec![true, true, false],
        "the rgb shift row reads 0.00 and did not say the pass is not in the frame"
    );
}

/// **The cut chip is on the feedback row and on neither of the others**, which
/// is the shape that keeps a press on the bloom row from reaching a parameter
/// bloom has not got.
#[test]
fn only_the_feedback_row_carries_a_cut_chip() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    let chips: Vec<bool> = it
        .fx
        .iter()
        .flatten()
        .map(|row| row.cut.is_some())
        .collect();
    assert_eq!(chips, vec![true, false, false]);
}

/// **Each row's knob asks for its own pass, at the value the track is at**, and
/// the feedback row's carries the cut beside the amount — because the amount
/// alone is not a picture.
///
/// The far end of each track is asked for, because that is the reading a drag
/// can produce that the row was not already at.
#[test]
fn each_row_emits_its_own_operation_with_the_value_in_range() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let view = view(MOCK);
    let it = row(&panel, &ctx, MOCK);
    let expected: [Operation; 3] = [
        Operation::SetFeedback {
            params: Feedback {
                amount: Feedback::MAX,
                cut: Cut::Mix,
            },
        },
        Operation::SetBloom {
            params: karakuri_operation::Bloom { amount: 1.0 },
        },
        Operation::SetRgbShift {
            params: karakuri_operation::RgbShift { amount: 1.0 },
        },
    ];
    for (fx, want) in it.fx.iter().flatten().zip(expected) {
        let press = at(fx.fader.knob.center());
        assert_eq!(
            claim(&mut panel, &ctx, &view, press),
            Claim::Panel,
            "the panel did not claim a press on the {} row's knob",
            fx.fx.name()
        );
        let grab = master(&ctx, panel.layout(), view.master_out, view.master_chain)
            .and_then(|row| row.grab(press))
            .expect("the derivation that drew the knob says what a press takes hold of");
        panel.grab(press, grab);
        let to = Point::new(fx.fader.track.max.x, press.y);
        assert_eq!(
            panel.moved(to),
            Some(Dragged::Fader(want)),
            "the {} row's track dragged to its far end asked for the wrong thing",
            fx.fx.name()
        );
        panel.released(None);
    }
}

/// **A press on the cut chip asks for the other cut, and for nothing else.**
/// The cycle is this crate's arithmetic over a closed list of two — the blend
/// chip's arrangement — and what it emits is where the pass is going rather
/// than a step.
#[test]
fn the_cut_chip_asks_for_the_other_cut_and_keeps_the_amount() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    let feedback = it.fx[0].expect("the feedback row");
    let chip = feedback.cut.expect("the feedback row draws a cut chip");
    assert_eq!(
        it.chip(at(chip.center())),
        Some(Operation::SetFeedback {
            params: Feedback {
                amount: 0.34 * Feedback::MAX,
                cut: Cut::Exit,
            },
        }),
        "the chip on a row reading the mix cut did not ask for the exit cut at the same amount"
    );
    // And nowhere else on the bay is a chip.
    assert_eq!(it.chip(at(feedback.fader.knob.center())), None);
    assert_eq!(it.chip(at(it.fader.knob.center())), None);
}

/// **A console with a level and no chain draws the out row and nothing under
/// it**, which is what a harness that has not written the chain yet gets — and
/// it is not the same as no bay at all.
#[test]
fn no_chain_behind_the_console_is_the_out_row_alone() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = master(&ctx, panel.layout(), Some(MOCK), None)
        .expect("the out row draws without a chain behind it");
    assert!(
        it.fx.iter().all(Option::is_none),
        "the Master bay drew effect rows for a console with no chain behind it"
    );
}
