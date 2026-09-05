//! **The Mixer bay's transition row: where it is drawn, and what its three
//! pills ask for.**
//!
//! `mixer.rs` is where a strip's rectangles are; `blend.rs` is the closest
//! model for what is here, because this row's pills are the blend chip's
//! affordance three times over — the pill cycles, and a press emits
//! `Operation::SetTransition` naming the setting it **arrived at**.
//!
//! **The cycle is the affordance and the operation is the destination**, which
//! is what P-0090 leaves to whoever draws a control: a pill that cycles is one
//! control emitting six, the operator sees a toggle and the vocabulary never
//! does. What is asserted here is a named destination per press, the wrap on
//! each of the three cycles, and that a press anywhere else in the bay is not
//! this row's.
//!
//! # The cycles are written out here rather than read off the crate
//!
//! `karakuri_operation::TransitionSetting` has no `ALL` and no `name`, so
//! there is no vocabulary list for this file to walk the way `blend.rs` walks
//! `BlendMode::ALL`. What the console cycles is a **curation** —
//! `view::WIPE_SHAPES`, `view::QUANTA` and `view::FADE_BEATS`, which are
//! private — and a test that read them back would be comparing the code with
//! itself. So the three lists below are the second copy on purpose, and they
//! are the copy the row is judged against.
//!
//! **None of it needs a device.** Laying the row out needs `egui`, because a
//! pill is as wide as the word in it — `mixer.rs`'s own opening — and what
//! comes out is `karakuri-operation`'s, which has no dependencies at all.
//!
//! # Where this stops
//!
//! At the operation. Nothing here turns one into a `Current::transition` and
//! runs a wipe with it: that is the harness's, `crates/karakuri/src/main.rs`,
//! where there is a deck (ADR-0156).

mod common;

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use common::{drawn_once, near, rect_of, showing, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Panel, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    mixer, transition, Go, Level, Mask, Strip, Tally, TransitionRow, TransitionSettings, View,
};
use karakuri_layout::{Hit, Point, Rect};
use karakuri_operation::{BlendMode, Operation, TransitionSetting, WipeKind};

/// **The six shapes the row's first pill cycles**, in order — the curation
/// `view::WIPE_SHAPES` holds, written again here because nothing in
/// `karakuri-operation` enumerates it. They are `karakuri-cli`'s `MASK_SHAPES`
/// pairs, which is the point of the curation: two surfaces stepping this
/// setting arrive at the same six places.
const SHAPES: [(WipeKind, f32); 6] = [
    (WipeKind::None, 0.0),
    (WipeKind::Linear, 0.0),
    (WipeKind::Linear, FRAC_PI_2),
    (WipeKind::Linear, FRAC_PI_4),
    (WipeKind::Linear, -FRAC_PI_4),
    (WipeKind::Radial, 0.0),
];

/// **The three grids the second pill cycles** — `karakuri-cli`'s `QUANTA`, in
/// its order: the next bar, the next beat, now.
const QUANTA: [f64; 3] = [4.0, 1.0, 0.0];

/// **The four lengths the third pill cycles** — `karakuri-cli`'s `FADE_BEATS`:
/// a bar, half a bar, two bars, and a cut.
const LENGTHS: [f64; 4] = [4.0, 2.0, 8.0, 0.0];

/// **The words the six shapes read**, in the same order — what the pill is as
/// wide as and what a reader sees. `no shape` and never `off`: that word is a
/// residency on this console and `preview_caption.rs` asserts it is painted
/// nowhere.
const SHAPE_WORDS: [&str; 6] = [
    "no shape",
    "left",
    "up",
    "diagonal",
    "back diagonal",
    "iris",
];
const QUANTUM_WORDS: [&str; 3] = ["next bar", "next beat", "now"];
const LENGTH_WORDS: [&str; 4] = ["4 beats", "2 beats", "8 beats", "cut"];

/// Three strips, so the bay above the row is drawn and a press on it can be
/// asked about. The values are `mixer.rs`'s mock read loosely; nothing here
/// reads any of them.
fn strips() -> Vec<Strip> {
    ["drift_night", "lattice_veil", "glass_shell"]
        .into_iter()
        .enumerate()
        .map(|(slot, name)| Strip {
            name: name.to_owned(),
            tally: Tally::Live,
            requested: Tally::Live,
            gain: 0.2 + 0.15 * slot as f32,
            gain_to: None,
            opacity: 0.8 - 0.15 * slot as f32,
            opacity_to: None,
            blend: BlendMode::ALL[slot % BlendMode::ALL.len()],
            mask: Mask::None,
            mask_angle: 0.0,
            level: Some(Level {
                mean: 0.5,
                peak: 0.6,
            }),
        })
        .collect()
}

/// A panel at a viewport, solved, with a context that has drawn once — the
/// pair `mixer.rs`, `blend.rs` and `fader.rs` all open with.
fn console(viewport: Rect) -> (Panel, egui::Context) {
    let mut panel = Panel::new(viewport.w, viewport.h);
    panel.solve();
    (panel, drawn_once())
}

/// A console showing `strips` with the transition row at `settings`.
fn showing_at(strips: &[Strip], settings: TransitionSettings) -> View {
    let mut view = showing(strips);
    apply(&mut view, settings);
    view
}

/// Walk `view`'s row to `settings` through the only door there is, and panic
/// where a setting is refused — a test that silently kept the old value would
/// be asserting about a row it did not set.
fn apply(view: &mut View, settings: TransitionSettings) {
    for setting in [
        TransitionSetting::WipeShape {
            kind: settings.kind,
            angle: settings.angle,
        },
        TransitionSetting::Quantum {
            beats: settings.quantum,
        },
        TransitionSetting::Length {
            beats: settings.length,
        },
    ] {
        view.set_transition(setting);
    }
    assert_eq!(
        view.transition(),
        settings,
        "the console refused a setting this test needs it to be on"
    );
}

/// The row, laid out. Panics where it is not drawn, which is the failure worth
/// reading.
fn row(panel: &Panel, ctx: &egui::Context, settings: TransitionSettings) -> TransitionRow {
    transition(ctx, panel.layout(), settings).expect("the mixer bay draws its transition row")
}

/// Settings at a named place in each of the three cycles.
fn place(shape: usize, quantum: usize, length: usize) -> TransitionSettings {
    TransitionSettings {
        kind: SHAPES[shape].0,
        angle: SHAPES[shape].1,
        quantum: QUANTA[quantum],
        length: LENGTHS[length],
    }
}

fn point(p: egui::Pos2) -> Point {
    Point::new(p.x, p.y)
}

/// Every text the console paints on one frame, with where it was painted —
/// `preview_caption.rs`'s helper, which is how *what is drawn* is asked
/// anywhere in this crate.
fn texts(view: &mut View, panel: &mut Panel) -> Vec<(egui::Pos2, String)> {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter_map(|c| match c.shape {
            egui::Shape::Text(at) => Some((at.pos, at.galley.text().to_owned())),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Where the row is
// ---------------------------------------------------------------------------

/// **The row is where the mock puts it**, and every number is read off
/// `style.css` rather than off the panel.
///
/// The transcription first, for `mixer.rs`'s reason (ADR-0177): everything
/// after it is a *relation* — this box is one padding under that one — and a
/// relation holds just as well with the padding transcribed wrong.
#[test]
fn the_row_is_the_mocks_own_box() {
    assert!(
        near(size::XFADE_PAD_TOP, 8.0),
        "`.xfade` is `padding: 8px 10px 10px`"
    );
    assert!(near(size::XFADE_PAD_X, 10.0), "`.xfade`'s side padding");
    assert!(near(size::XFADE_PAD_BOTTOM, 10.0), "`.xfade`'s bottom");
    assert!(near(size::XFADE_GAP, 7.0), "`.xfade` is `gap: 7px`");
    assert!(near(size::XROW_GAP, 8.0), "`.xrow` is `gap: 8px`");
    assert!(near(size::HAIRLINE, 1.0), "`.xfade`'s `border-top` is 1px");
    // A pill at the console's own type size inside `.pill`'s border, which is
    // the box `.mini` and `.rend` are and is the 18.5 `lib.rs` derived this
    // bay's height from.
    assert!(
        near(size::XPILL_H, 18.5),
        "a pill is {} tall",
        size::XPILL_H
    );
    assert!(
        near(size::XFADE_H, 37.5),
        "`.xfade` is {} tall",
        size::XFADE_H
    );

    let (panel, ctx) = console(SMALLEST);
    let region = rect_of(panel.layout(), "mixer");
    let at = row(&panel, &ctx, TransitionSettings::START);

    // The block: directly under `.mixer-strips` — the strips' own row plus the
    // padding below it — the full width of the bay, and exactly `XFADE_H`
    // tall rather than whatever the bay has left over.
    let strips_bottom = region.y + size::HEAD_H + size::STRIPS_PAD * 2.0 + size::STRIP_H;
    assert!(near(at.rect.min.y, strips_bottom));
    assert!(near(at.rect.height(), size::XFADE_H));
    assert!(near(at.rect.min.x, region.x));
    assert!(near(at.rect.width(), region.w));

    // And it is inside the bay, with room to spare: the 23.5 the crossfader
    // took with it when the mixer was decided to have none.
    assert!(
        at.rect.max.y <= region.y + region.h + common::EPS,
        "the transition row runs off the bottom of the bay"
    );

    // The three pills: one hairline and one top padding down from the block,
    // each `XPILL_H` tall, the first one side padding in, the next two one
    // `.xrow` gap after the one before, and the last inside the padding on the
    // other side.
    let top = at.rect.min.y + size::HAIRLINE + size::XFADE_PAD_TOP;
    let pills = [at.shape, at.quantum, at.length];
    for (index, pill) in pills.iter().enumerate() {
        assert!(
            near(pill.min.y, top),
            "pill {index} is not on the row's line"
        );
        assert!(
            near(pill.height(), size::XPILL_H),
            "pill {index} is {} tall",
            pill.height()
        );
        assert!(
            pill.width() > size::PILL_PAD_X * 2.0,
            "pill {index} has no word in it"
        );
    }
    assert!(near(at.shape.min.x, at.rect.min.x + size::XFADE_PAD_X));
    assert!(near(at.quantum.min.x, at.shape.max.x + size::XROW_GAP));
    assert!(near(at.length.min.x, at.quantum.max.x + size::XROW_GAP));
    assert!(
        at.length.max.x <= at.rect.max.x - size::XFADE_PAD_X + common::EPS,
        "the last pill runs into `.xfade`'s own padding"
    );

    // The strips are above it and nothing of the two overlaps: the row is
    // under `.mixer-strips` and not in it.
    let strips = strips();
    let bay = mixer(&ctx, panel.layout(), &strips).expect("the strips");
    assert!(
        bay.strip(0).rect.max.y <= at.rect.min.y + common::EPS,
        "a strip reaches into the transition row"
    );
}

/// **The pills are drawn where they are pressed**, at the widest word each
/// cycle has.
///
/// The word a pill reads is painted inside the rectangle the hit test answers
/// for, which is this crate's rule for every control it has: the derivation
/// that draws a control is asked again rather than copied, so a pill an
/// operator sees and a pill they click cannot come apart.
///
/// **Every place in all three cycles**, because a pill is as wide as its own
/// word and the words are not the same length: a row laid out from one word
/// and painted with another would come apart only at the value where the two
/// widths differ.
#[test]
fn every_word_of_every_cycle_is_painted_in_its_own_pill() {
    let strips = strips();
    let mut measured = 0;
    for (shape, shape_word) in SHAPE_WORDS.into_iter().enumerate() {
        for (quantum, quantum_word) in QUANTUM_WORDS.into_iter().enumerate() {
            for (length, length_word) in LENGTH_WORDS.into_iter().enumerate() {
                let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
                panel.solve();
                let settings = place(shape, quantum, length);
                let mut view = showing_at(&strips, settings);
                let ctx = drawn_once();
                let at = row(&panel, &ctx, settings);
                let painted = texts(&mut view, &mut panel);
                for (pill, word) in [
                    (at.shape, shape_word),
                    (at.quantum, quantum_word),
                    (at.length, length_word),
                ] {
                    measured += 1;
                    assert!(
                        painted
                            .iter()
                            .any(|(pos, text)| text == word && pill.contains(*pos)),
                        "`{word}` is not painted inside the pill {pill:?} that answers for it"
                    );
                }
            }
        }
    }
    assert_eq!(
        measured,
        SHAPES.len() * QUANTA.len() * LENGTHS.len() * 3,
        "this test measured no pills, so it is asserting nothing"
    );
}

/// **The row is drawn with no deck behind the console.**
///
/// `mixer` answers `None` there and draws no strips, because six readings a
/// slot with no slot to read is a row of zeroes. These three are not readings
/// — they are the console's own pointer and always have a value — so the row
/// survives it, and a press on it is still the panel's.
#[test]
fn a_console_with_no_deck_still_draws_the_row() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let strips: Vec<Strip> = Vec::new();
    assert!(
        mixer(&ctx, panel.layout(), &strips).is_none(),
        "a console with no deck drew strips, so this test is not asserting what it says"
    );
    let at = row(&panel, &ctx, TransitionSettings::START);
    let view = View::new(Room::Day);
    assert_eq!(view.transition(), TransitionSettings::START);
    for pill in [at.shape, at.quantum, at.length] {
        assert!(at.owns(point(pill.center())), "a pill answers for nothing");
        assert_eq!(
            claim(&mut panel, &ctx, &view, point(pill.center())),
            Claim::Panel,
            "a pill of a deckless console is not the panel's"
        );
    }
}

// ---------------------------------------------------------------------------
// What each pill asks for
// ---------------------------------------------------------------------------

/// **A press on the shape pill asks for the next shape, and the last wraps to
/// the first.**
///
/// The operation names a **destination** and never a step, which is the whole
/// of what P-0090 asks of a control that cycles. The wrap is not a case in the
/// assertion: the loop's last step is the sixth shape and the expected answer
/// is the first, reached by the same modulo every other step uses.
#[test]
fn a_press_on_the_shape_pill_names_the_next_shape_and_wraps() {
    let strips = strips();
    for (step, (kind, angle)) in SHAPES.into_iter().enumerate() {
        let (panel, ctx) = console(PLAUSIBLE);
        let (want_kind, want_angle) = SHAPES[(step + 1) % SHAPES.len()];
        let settings = place(step, 0, 0);
        let at = row(&panel, &ctx, settings);
        assert_eq!(
            at.shape(point(at.shape.center())),
            Some(Operation::SetTransition {
                setting: TransitionSetting::WipeShape {
                    kind: want_kind,
                    angle: want_angle,
                },
            }),
            "a press on a pill reading `{}` did not ask for `{}`",
            SHAPE_WORDS[step],
            SHAPE_WORDS[(step + 1) % SHAPES.len()]
        );
        // And the console takes it, which is what makes the cycle a cycle
        // rather than six answers nobody can act on.
        let mut view = showing_at(&strips, settings);
        assert!(
            view.set_transition(TransitionSetting::WipeShape {
                kind: want_kind,
                angle: want_angle
            }),
            "the console refused the shape its own pill asked for"
        );
        assert_eq!(
            (view.transition().kind, view.transition().angle),
            (want_kind, want_angle)
        );
        assert_eq!((kind, angle), (settings.kind, settings.angle));
    }
}

/// **A press on the quantum pill asks for the next grid, and the last wraps to
/// the first.** [`a_press_on_the_shape_pill_names_the_next_shape_and_wraps`]
/// one pill along.
#[test]
fn a_press_on_the_quantum_pill_names_the_next_grid_and_wraps() {
    for step in 0..QUANTA.len() {
        let (panel, ctx) = console(PLAUSIBLE);
        let want = QUANTA[(step + 1) % QUANTA.len()];
        let at = row(&panel, &ctx, place(0, step, 0));
        assert_eq!(
            at.quantum(point(at.quantum.center())),
            Some(Operation::SetTransition {
                setting: TransitionSetting::Quantum { beats: want },
            }),
            "a press on a pill reading `{}` did not ask for `{}`",
            QUANTUM_WORDS[step],
            QUANTUM_WORDS[(step + 1) % QUANTA.len()]
        );
    }
}

/// **A press on the length pill asks for the next length, and the last wraps
/// to the first.** The cut is one of the four and is reached by the same
/// modulo, so a cycle that skipped it would fail here.
#[test]
fn a_press_on_the_length_pill_names_the_next_length_and_wraps() {
    for step in 0..LENGTHS.len() {
        let (panel, ctx) = console(PLAUSIBLE);
        let want = LENGTHS[(step + 1) % LENGTHS.len()];
        let at = row(&panel, &ctx, place(0, 0, step));
        assert_eq!(
            at.length(point(at.length.center())),
            Some(Operation::SetTransition {
                setting: TransitionSetting::Length { beats: want },
            }),
            "a press on a pill reading `{}` did not ask for `{}`",
            LENGTH_WORDS[step],
            LENGTH_WORDS[(step + 1) % LENGTHS.len()]
        );
    }
}

/// **Each pill answers for its own setting and for neither of the other
/// two**, so a press meant for the length cannot change the shape.
///
/// The three sit in one row eight pixels apart and each is asked about all
/// three points, which is `blend.rs`'s *the two neighbours that are controls*
/// with a third neighbour added.
#[test]
fn a_pill_answers_for_its_own_setting_and_no_other() {
    let (panel, ctx) = console(PLAUSIBLE);
    let at = row(&panel, &ctx, place(5, 2, 3));
    let probes = [
        (at.shape.center(), "the shape pill"),
        (at.quantum.center(), "the quantum pill"),
        (at.length.center(), "the length pill"),
    ];
    for (index, (probe, what)) in probes.into_iter().enumerate() {
        let asked = [
            at.shape(point(probe)),
            at.quantum(point(probe)),
            at.length(point(probe)),
        ];
        for (which, answer) in asked.iter().enumerate() {
            assert_eq!(
                answer.is_some(),
                which == index,
                "{what} was answered for by hit test {which}"
            );
        }
    }
}

/// **Pressing the row round every cycle returns it to where it started, and
/// visits every place on the way.**
///
/// This is the half a per-press assertion cannot make: a cycle that skipped a
/// value and one that repeated one both answer a plausible destination at
/// every step, and only the walk says the row has been everywhere and come
/// back. It goes through [`View::set_transition`] rather than through the
/// table, so what is walked is the door the panel actually has.
#[test]
fn the_three_cycles_wrap_and_visit_every_place() {
    let strips = strips();
    let (panel, ctx) = console(PLAUSIBLE);

    let mut view = showing_at(&strips, TransitionSettings::START);
    let mut seen = Vec::new();
    for _ in 0..SHAPES.len() {
        let settings = view.transition();
        seen.push((settings.kind, settings.angle));
        let at = row(&panel, &ctx, settings);
        let Some(Operation::SetTransition { setting }) = at.shape(point(at.shape.center())) else {
            panic!("the shape pill asked for nothing");
        };
        assert!(view.set_transition(setting), "a step moved nothing");
    }
    assert_eq!(
        (view.transition().kind, view.transition().angle),
        (SHAPES[0].0, SHAPES[0].1),
        "six presses on the shape pill did not come back to where they started"
    );
    for (index, place) in SHAPES.into_iter().enumerate() {
        assert_eq!(seen[index], place, "the shape cycle is not the curated six");
    }

    let mut view = showing_at(&strips, TransitionSettings::START);
    let mut seen = Vec::new();
    for _ in 0..QUANTA.len() {
        let settings = view.transition();
        seen.push(settings.quantum);
        let at = row(&panel, &ctx, settings);
        let Some(Operation::SetTransition { setting }) = at.quantum(point(at.quantum.center()))
        else {
            panic!("the quantum pill asked for nothing");
        };
        assert!(view.set_transition(setting), "a step moved nothing");
    }
    assert_eq!(view.transition().quantum, QUANTA[0]);
    assert_eq!(seen, QUANTA, "the quantum cycle is not the curated three");

    let mut view = showing_at(&strips, TransitionSettings::START);
    let mut seen = Vec::new();
    for _ in 0..LENGTHS.len() {
        let settings = view.transition();
        seen.push(settings.length);
        let at = row(&panel, &ctx, settings);
        let Some(Operation::SetTransition { setting }) = at.length(point(at.length.center()))
        else {
            panic!("the length pill asked for nothing");
        };
        assert!(view.set_transition(setting), "a step moved nothing");
    }
    assert_eq!(view.transition().length, LENGTHS[0]);
    assert_eq!(seen, LENGTHS, "the length cycle is not the curated four");
}

/// **A setting off the cycles is refused, and one that names where the row
/// already is moves nothing.**
///
/// The second half is P-0091 at a control: a caller repaints on a move and not
/// on a press, so a press that changed nothing costs no frame. The first is
/// `View::select`'s rule — the row is three capsules and each reads the word
/// its cycle gives, so a value off the cycle would be a pill with nothing to
/// say.
#[test]
fn a_setting_off_the_cycles_is_refused_and_one_that_moves_nothing_says_so() {
    let strips = strips();
    let mut view = showing_at(&strips, TransitionSettings::START);

    // An angle between two curated ones, a quantum and a length that are not
    // on either list.
    for setting in [
        TransitionSetting::WipeShape {
            kind: WipeKind::Linear,
            angle: 0.3,
        },
        TransitionSetting::Quantum { beats: 3.0 },
        TransitionSetting::Length { beats: 5.5 },
    ] {
        assert!(
            !view.set_transition(setting),
            "{setting:?} is not on any of the row's cycles and was taken anyway"
        );
    }
    assert_eq!(
        view.transition(),
        TransitionSettings::START,
        "a refused setting moved the row"
    );

    // And a setting the row is already on.
    for setting in [
        TransitionSetting::WipeShape {
            kind: SHAPES[0].0,
            angle: SHAPES[0].1,
        },
        TransitionSetting::Quantum { beats: QUANTA[0] },
        TransitionSetting::Length { beats: LENGTHS[0] },
    ] {
        assert!(
            !view.set_transition(setting),
            "{setting:?} is where the row already was and was reported as a move"
        );
    }

    // The guard on both: a setting that *is* on a cycle and is somewhere else
    // is taken and says so. Without it this passes on a setter that refuses
    // everything.
    assert!(view.set_transition(TransitionSetting::Quantum { beats: QUANTA[1] }));
    assert_eq!(view.transition().quantum, QUANTA[1]);
}

// ---------------------------------------------------------------------------
// A control claims what it acts on and no more
// ---------------------------------------------------------------------------

/// **A press elsewhere in the bay asks this row for nothing and is not claimed
/// by it.**
///
/// `input`'s rule 4: *"a control claims what it acts on and no more."* The
/// ground either side of the pills, the gaps between them and the card under
/// the row are all painted by the console and none of them is a control, so
/// `egui` gets the event — which owns no widget there either, so the two
/// answers are the same nothing, arrived at without the panel claiming a press
/// it would throw away.
///
/// **Both halves, because either alone is satisfiable by the wrong code.** A
/// row that emitted nothing but was claimed would take presses it does nothing
/// with; one that emitted from anywhere would change what the next wipe means
/// from a press on the card beside it.
///
/// **A strip's own controls are asserted the other way round**, exactly as
/// `blend.rs` does with the tally and the mask: the panel claims each, and
/// this row must answer nothing for it.
#[test]
fn a_press_off_the_pills_asks_for_nothing_and_is_not_claimed() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let strips = strips();
    let settings = place(5, 0, 2);
    let mut showing = showing_at(&strips, settings);
    let at = row(&panel, &ctx, settings);
    let bay = mixer(&ctx, panel.layout(), &strips).expect("the strips");

    let probes = [
        // Two pixels left of the first pill, and **not** the middle of
        // `.xfade`'s own padding: that point is five in from the bay's left
        // edge and inside the pane boundary's `GRAB`, so rule 3 would answer
        // *the panel's* before this row was asked anything at all.
        (
            egui::pos2(at.shape.min.x - 2.0, at.shape.center().y),
            "the padding left of the first pill",
        ),
        (
            egui::pos2(at.shape.max.x + size::XROW_GAP * 0.5, at.shape.center().y),
            "the gap between the shape and the quantum",
        ),
        (
            egui::pos2(
                at.quantum.max.x + size::XROW_GAP * 0.5,
                at.quantum.center().y,
            ),
            "the gap between the quantum and the length",
        ),
        (
            egui::pos2(at.length.max.x + 4.0, at.length.center().y),
            "the card right of the last pill",
        ),
        (
            egui::pos2(at.shape.center().x, at.rect.min.y + size::HAIRLINE * 0.5),
            "the rule along the top of the row",
        ),
        (
            egui::pos2(at.shape.center().x, at.shape.max.y + 2.0),
            "the padding under the pills",
        ),
    ];
    for (probe, what) in probes {
        for pill in [at.shape, at.quantum, at.length] {
            assert!(
                !pill.contains(probe),
                "{what} is inside a pill, so this probe asserts nothing"
            );
        }
        assert!(!at.owns(point(probe)), "{what} was claimed by the row");
        assert_eq!(
            claim(&mut panel, &ctx, &showing, point(probe)),
            Claim::Egui,
            "{what} is being claimed as a control the panel acts on"
        );
    }

    // **The strip above, which is a control and is not this row's.** A hit
    // test that reached up would change what the next wipe means from a press
    // meant for a blend mode.
    for (probe, what) in [
        (bay.strip(0).blend.center(), "a strip's blend chip"),
        (bay.strip(0).mask.center(), "a strip's mask mini"),
        (bay.strip(1).name.center(), "a strip's name"),
    ] {
        assert!(!at.owns(point(probe)), "{what} was claimed by the row");
        assert_eq!(
            at.shape(point(probe)),
            None,
            "{what} asked the wipe shape to change"
        );
    }

    // The guard: the four capsules do both of the things the probes above do
    // neither of. Without this the test passes on a row that was never a
    // control at all.
    for (pill, which) in [
        (at.shape, "shape"),
        (at.quantum, "quantum"),
        (at.length, "length"),
        (at.go, "go"),
    ] {
        let on = pill.center();
        assert!(at.owns(point(on)), "the {which} pill asked for nothing");
        assert_eq!(
            claim(&mut panel, &ctx, &showing, point(on)),
            Claim::Panel,
            "the {which} pill is not the panel's, so it is drawn where it cannot be clicked"
        );
    }

    // And the row moves with the console's own value, which is what says the
    // probes above were taken against the row that was drawn: at a different
    // shape the pills are different widths.
    apply(&mut showing, place(4, 0, 2));
    let wider = row(&panel, &ctx, showing.transition());
    assert_ne!(
        wider.quantum.min.x, at.quantum.min.x,
        "`back diagonal` and `iris` laid the row out identically, so this file's widths say \
         nothing"
    );
}

/// **No pill of the transition row is inside a boundary's [`GRAB`].**
///
/// `input`'s rule 3 comes before rule 4, so a control under a boundary's grab
/// band is a control that cannot be clicked, with nothing on screen saying so.
/// This row is the **lowest** thing in the Mixer bay — 24 pixels above the
/// boundary between the mixer and the master chain — where the blend chip
/// measured for this is at the bottom of a *strip*, so it is a different
/// clearance against the same boundary and is measured rather than inherited.
///
/// It asks `Layout::hit` directly as well as `claim`, which is ADR-0185's
/// caught test: `claim` says *the panel's* for a boundary **and** for a
/// control, so a version of this that only asked `claim` would pass with
/// `GRAB` widened to 60.
#[test]
fn no_pill_is_inside_a_boundarys_grab() {
    let strips = strips();
    for viewport in [PLAUSIBLE, SMALLEST] {
        let (mut panel, ctx) = console(viewport);
        let settings = place(4, 1, 2);
        let showing = showing_at(&strips, settings);
        let at = row(&panel, &ctx, settings);
        for (pill, which) in [
            (at.shape, "shape"),
            (at.quantum, "quantum"),
            (at.length, "length"),
            // **The `go` capsule is the one at the other end of the row**, so
            // its clearance is off the bay's right edge rather than its left
            // and is measured rather than inherited from the three: it is
            // `.xfade`'s own `XFADE_PAD_X` of 10 against a `GRAB` of 6.
            (at.go, "go"),
        ] {
            for probe in [
                pill.left_top(),
                pill.right_top(),
                pill.left_bottom(),
                pill.right_bottom(),
                pill.center(),
            ] {
                assert!(
                    !matches!(panel.layout().hit(point(probe), GRAB), Hit::Divider { .. }),
                    "a boundary grabs {probe:?}, which is on the {which} pill — the control is \
                     dead there, and `input`'s rule 3 is what would have to change"
                );
                assert_eq!(
                    claim(&mut panel, &ctx, &showing, point(probe)),
                    Claim::Panel,
                    "the {which} pill is not the panel's at {probe:?}"
                );
            }
        }

        // The guard: the ground under the bay *is* inside a boundary's grab,
        // which is what says the answers above are the clearance rather than
        // the grab having gone missing.
        let region = rect_of(panel.layout(), "mixer");
        let below = Point::new(region.x + region.w * 0.5, region.y + region.h + GRAB * 0.5);
        assert!(
            matches!(panel.layout().hit(below, GRAB), Hit::Divider { .. }),
            "the ground under the mixer is not in the grab of the boundary there, so this test \
             is no longer measuring the clearance it was written for"
        );
        assert!(
            !at.owns(below),
            "a point in the ground under the bay pressed a pill"
        );
    }
}

// ---------------------------------------------------------------------------
// The `go` capsule
// ---------------------------------------------------------------------------

/// **The `go` capsule is at the right end of the row**, which is what `.sep`'s
/// `flex: 1` puts it — measured back from `.xfade`'s own padding on that side
/// rather than forward from the length pill.
///
/// The three settings grow with their words and this one does not move, which
/// is the property a separator has and a fourth pill laid end to end would
/// not: at `no shape · now · cut` and at `back diagonal · next beat · 8 beats`
/// the capsule is in the same place, and the gap between it and the length
/// pill is what changes.
#[test]
fn the_go_capsule_sits_against_the_rows_right_padding() {
    let (panel, ctx) = console(PLAUSIBLE);
    let narrow = row(&panel, &ctx, place(0, 2, 3));
    let wide = row(&panel, &ctx, place(4, 1, 2));

    for (at, which) in [(narrow, "the narrowest words"), (wide, "the widest")] {
        assert!(
            near(at.go.max.x, at.rect.max.x - size::XFADE_PAD_X),
            "with {which} the `go` capsule does not finish on `.xfade`'s right padding: \
             {} against {}",
            at.go.max.x,
            at.rect.max.x - size::XFADE_PAD_X
        );
        assert!(
            near(
                at.go.min.y,
                at.rect.min.y + size::HAIRLINE + size::XFADE_PAD_TOP
            ),
            "the `go` capsule is not on the row's line"
        );
        assert!(
            near(at.go.height(), size::XPILL_H),
            "the `go` capsule is {} tall and a pill on this row is {}",
            at.go.height(),
            size::XPILL_H
        );
        assert!(
            at.length.max.x + size::XROW_GAP <= at.go.min.x + common::EPS,
            "`.sep` is not a gap: the length pill runs into the `go` capsule"
        );
    }

    // The whole of what `.sep` buys, stated as the difference: the capsule
    // does not move when the words to its left do, and the separator is what
    // takes up the slack. Without this the two halves above are satisfied by a
    // fourth pill laid end to end at a window this wide.
    assert!(
        near(narrow.go.min.x, wide.go.min.x),
        "the `go` capsule moved with the words beside it, so it is laid end to end rather \
         than pushed to the end by `.sep`"
    );
    assert!(
        wide.length.max.x > narrow.length.max.x,
        "the two settings this test compares laid the row out identically, so it is \
         asserting nothing"
    );

    // And the word is painted inside the capsule that answers for it, which is
    // this crate's rule for every control it has.
    let strips = strips();
    let settings = place(5, 0, 2);
    let mut panel = Panel::new(PLAUSIBLE.w, PLAUSIBLE.h);
    panel.solve();
    let ctx = drawn_once();
    let at = row(&panel, &ctx, settings);
    let mut view = showing_at(&strips, settings);
    let painted = texts(&mut view, &mut panel);
    assert!(
        painted
            .iter()
            .any(|(pos, text)| text == "go" && at.go.contains(*pos)),
        "`go` is not painted inside the capsule that answers for it"
    );
}

/// **A press on `go` asks for a wipe naming the addressed deck and the next
/// one round**, and the last deck wraps to the first.
///
/// `Operation::Wipe` carries *the deck being covered* and *the deck arriving
/// over it*, and which two those are is a surface's translation — the
/// vocabulary carries both decks precisely so that it is not the operation's.
/// This row names them `karakuri-cli`'s way: the selection is covered and the
/// next slot round arrives over it, which is ADR-0259's *fade, crossfade and
/// wipe are acts on the addressed strip*.
///
/// **The wrap is not a case in the assertion**: the loop's last step is the
/// last strip and the expected answer is the first, reached by the same modulo
/// every other step uses.
#[test]
fn a_press_on_go_covers_the_addressed_deck_with_the_next_one_round() {
    let strips = strips();
    let (panel, ctx) = console(PLAUSIBLE);
    let settings = place(5, 0, 2);
    let at = row(&panel, &ctx, settings);
    let decks = strips.len();
    assert!(
        decks >= 2,
        "a one-strip mixer cannot wipe and this test needs one that can"
    );
    for from in 0..decks {
        let mut view = showing_at(&strips, settings);
        // `View::select` answers whether the selection *moved*, so a `false`
        // for deck A is a press that changed nothing rather than a refusal —
        // what this test needs is where the pointer ended up.
        view.select(from as u8);
        assert_eq!(
            view.selection(),
            from as u8,
            "the console refused a selection this test needs it to be on"
        );
        assert_eq!(
            at.go(point(at.go.center()), view.selection(), view.mixer.len()),
            Some(Go::Wipe(Operation::Wipe {
                from: from as u8,
                to: ((from + 1) % decks) as u8,
            })),
            "a press on `go` with deck {from} addressed did not cover it with deck {}",
            (from + 1) % decks
        );
    }
}

/// **A press on `go` with no shape chosen is refused, and the refusal says
/// which of the two it is.**
///
/// The refusal is the *surface's* and not the conversion's:
/// `Operation::Wipe` says *"Refused with no shape chosen"* at its own
/// definition, `karakuri-operation-record`'s three answers contain no refusal
/// at all, and the shape is this console's own setting — so this is the only
/// place a wipe with nothing to move can be turned away.
/// `karakuri-cli`'s `c` is the precedent and refuses the same two, in this
/// order.
///
/// **`WipeKind::None` is the first entry of the cycle and is where a run
/// begins**, so this is the state a console nobody has pressed anything on is
/// in rather than one a test had to arrange.
#[test]
fn a_press_on_go_with_no_shape_chosen_is_refused() {
    let strips = strips();
    let (panel, ctx) = console(PLAUSIBLE);
    let view = showing_at(&strips, TransitionSettings::START);
    assert_eq!(
        view.transition().kind,
        WipeKind::None,
        "a run does not begin with no shape chosen, so this test is not in the state it says"
    );
    let at = row(&panel, &ctx, view.transition());
    assert_eq!(
        at.go(point(at.go.center()), view.selection(), view.mixer.len()),
        Some(Go::NoShape),
        "a press on `go` with the shape pill reading `no shape` was not refused for it"
    );
    assert!(
        !at.runs(view.mixer.len()),
        "the capsule is drawn lit over a press that is refused"
    );

    // The guard: the same press with a shape chosen and nothing else changed
    // is a wipe. Without it this passes on a control that refuses everything.
    let armed = place(5, 0, 2);
    let mut chosen = showing_at(&strips, armed);
    chosen.select(0);
    assert_eq!(chosen.selection(), 0);
    let at = row(&panel, &ctx, armed);
    assert_eq!(
        at.go(
            point(at.go.center()),
            chosen.selection(),
            chosen.mixer.len()
        ),
        Some(Go::Wipe(Operation::Wipe { from: 0, to: 1 })),
        "the same press with an `iris` chosen is still not a wipe"
    );
    assert!(
        at.runs(chosen.mixer.len()),
        "the capsule is not lit over a press that runs"
    );
}

/// **A press on `go` with one strip in the bay is refused, and it is the other
/// refusal.**
///
/// `karakuri-cli`'s *"a wipe needs somewhere to come from — this deck holds
/// one slot"*, and it is asked before the shape for that program's reason: a
/// deck with nowhere to go is refused whatever is armed.
///
/// **A console with no deck at all is the same answer**, which is the row
/// being drawn without one ([`a_console_with_no_deck_still_draws_the_row`]):
/// the capsule is there, it is not lit, and a press on it says why rather than
/// reaching nothing.
#[test]
fn a_press_on_go_with_nowhere_to_come_from_is_refused() {
    let (panel, ctx) = console(PLAUSIBLE);
    let armed = place(5, 0, 2);
    let one: Vec<Strip> = strips().into_iter().take(1).collect();
    for (mixer, what) in [
        (one, "a one-strip mixer"),
        (Vec::new(), "a console with no deck"),
    ] {
        let view = showing_at(&mixer, armed);
        let at = row(&panel, &ctx, armed);
        assert!(
            view.transition().armed(),
            "{what} is not armed, so this test would be refused for the shape instead"
        );
        assert_eq!(
            at.go(point(at.go.center()), view.selection(), view.mixer.len()),
            Some(Go::NoOtherDeck),
            "a press on `go` at {what} was not refused for having nowhere to come from"
        );
        assert!(!at.runs(view.mixer.len()), "the capsule is lit at {what}");
    }

    // The guard: two strips and the same settings run. Without it this passes
    // on a control that refuses every deck count.
    let two: Vec<Strip> = strips().into_iter().take(2).collect();
    let view = showing_at(&two, armed);
    let at = row(&panel, &ctx, armed);
    assert_eq!(
        at.go(point(at.go.center()), view.selection(), view.mixer.len()),
        Some(Go::Wipe(Operation::Wipe { from: 0, to: 1 })),
        "two strips is somewhere to come from and the press was still refused"
    );
}

/// **The `go` capsule answers for itself and for none of the three settings,
/// and none of them answers for it.**
///
/// [`a_pill_answers_for_its_own_setting_and_no_other`] with the fourth capsule
/// added, and it is the direction that matters most on this row: a press meant
/// for a length that ran a wipe would put a deck on air.
#[test]
fn the_go_capsule_answers_for_itself_and_no_setting_answers_for_it() {
    let strips = strips();
    let (panel, ctx) = console(PLAUSIBLE);
    let settings = place(5, 2, 3);
    let view = showing_at(&strips, settings);
    let at = row(&panel, &ctx, settings);

    let on_go = point(at.go.center());
    assert_eq!(at.shape(on_go), None, "a press on `go` asked for a shape");
    assert_eq!(at.quantum(on_go), None, "a press on `go` asked for a grid");
    assert_eq!(at.length(on_go), None, "a press on `go` asked for a length");
    assert!(at.owns(on_go), "the `go` capsule is not the row's");

    for (probe, what) in [
        (at.shape.center(), "the shape pill"),
        (at.quantum.center(), "the quantum pill"),
        (at.length.center(), "the length pill"),
        (
            egui::pos2(at.go.min.x - size::XROW_GAP * 0.5, at.go.center().y),
            "`.sep`, which is ground and not a control",
        ),
    ] {
        assert_eq!(
            at.go(point(probe), view.selection(), view.mixer.len()),
            None,
            "{what} ran a wipe"
        );
    }
}
