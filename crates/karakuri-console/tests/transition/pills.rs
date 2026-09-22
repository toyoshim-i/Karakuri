use super::transition_common::*;

// ---------------------------------------------------------------------------
// Where the row is
// ---------------------------------------------------------------------------

/// The row is where the mock puts it, and every number is read off `style.css`
/// rather than off the panel.
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

/// The pills are drawn where they are pressed, at the widest word each cycle
/// has.
///
/// The word a pill reads is painted inside the rectangle the hit test answers
/// for, which is this crate's rule for every control it has: the derivation
/// that draws a control is asked again rather than copied, so a pill an
/// operator sees and a pill they click cannot come apart.
///
/// Every place in all three cycles, because a pill is as wide as its own word
/// and the words are not the same length: a row laid out from one word and
/// painted with another would come apart only at the value where the two widths
/// differ.
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

/// The row is drawn with no deck behind the console.
///
/// `mixer` answers `None` there and draws no strips, because six readings a
/// slot with no slot to read is a row of zeroes. These three are not readings —
/// they are the console's own pointer and always have a value — so the row
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

/// A press on the shape pill asks for the next shape, and the last wraps to the
/// first.
///
/// The operation names a destination and never a step, which is the whole of
/// what P-0090 asks of a control that cycles. The wrap is not a case in the
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

/// A press on the quantum pill asks for the next grid, and the last wraps to
/// the first. [`a_press_on_the_shape_pill_names_the_next_shape_and_wraps`] one
/// pill along.
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

/// A press on the length pill asks for the next length, and the last wraps to
/// the first. The cut is one of the four and is reached by the same modulo, so
/// a cycle that skipped it would fail here.
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

/// Each pill answers for its own setting and for neither of the other two, so a
/// press meant for the length cannot change the shape.
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

/// Pressing the row round every cycle returns it to where it started, and
/// visits every place on the way.
///
/// This is the half a per-press assertion cannot make: a cycle that skipped a
/// value and one that repeated one both answer a plausible destination at every
/// step, and only the walk says the row has been everywhere and come back. It
/// goes through [`View::set_transition`] rather than through the table, so what
/// is walked is the door the panel actually has.
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

/// A setting off the cycles is refused, and one that names where the row
/// already is moves nothing.
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
