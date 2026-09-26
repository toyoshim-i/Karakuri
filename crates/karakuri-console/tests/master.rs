//! Master bay output fader: layout, boundary clearance, drag tracking, and `SetMasterOut` emission (ADR-0224).

mod common;

use common::{at, console, near, rect_of, PLAUSIBLE, SMALLEST};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::{Dragged, Knob, Panel, Released, GRAB};
use karakuri_console::room::{size, Room};
use karakuri_console::view::{
    master, AddChoice, AddChoices, Added, Chain, ChainSlot, MasterRow, SlotParam, View,
};
use karakuri_layout::Point;
use karakuri_operation::{ChainParam, Cut, Operation};

/// The mock's own level: `out 1.00`, which is what `docs/manual/console.html`'s
/// Master bay draws and what `karakuri_engine::deck::Deck` comes up at.
const MOCK: f32 = 1.0;

/// A view with an engine behind it and that level in front of it — what
/// `View::draw` paints from and what `claim` hit-tests, one value.
fn view(out: f32) -> View {
    let mut view = View::new(Room::Day);
    view.master_out = Some(out);
    view.master_chain = Some(mock_chain());
    view.chain_add = offers();
    view
}

/// Mock processing chain with feedback and bloom effects under the output row.
fn mock_chain() -> Chain {
    Chain {
        slots: vec![
            ChainSlot {
                name: "feedback".to_owned(),
                cut: Some(Cut::Mix),
                params: vec![SlotParam {
                    key: "amount".to_owned(),
                    range: [0.0, 0.95],
                    value: 0.34,
                    default: 0.0,
                }],
            },
            ChainSlot {
                name: "bloom".to_owned(),
                cut: None,
                params: vec![SlotParam {
                    key: "amount".to_owned(),
                    range: [0.0, 1.0],
                    value: 0.60,
                    default: 0.0,
                }],
            },
        ],
    }
}

/// What the library offers `+ add`: one `kind L5` procedure that declares
/// `retains` and one that does not. Both of an add's cuts are reachable.
fn offers() -> Vec<AddChoice> {
    vec![
        AddChoice {
            procedure: "sha256:feed".to_owned(),
            words: "feedback".to_owned(),
            retains: true,
        },
        AddChoice {
            procedure: "sha256:b100".to_owned(),
            words: "bloom".to_owned(),
            retains: false,
        },
    ]
}

/// The chooser, with the card up — every test that is not about the card.
fn shut() -> AddChoices {
    AddChoices {
        items: offers(),
        open: false,
    }
}

/// The chooser with its card down.
fn down() -> AddChoices {
    AddChoices {
        items: offers(),
        open: true,
    }
}

/// The laid-out row at that level, on a solved console.
fn row(panel: &Panel, ctx: &egui::Context, out: f32) -> MasterRow {
    master(ctx, panel.layout(), Some(out), Some(&mock_chain()), &shut())
        .expect("the Master bay draws its out row")
}

// ---------------------------------------------------------------------------
// Where the control is
// ---------------------------------------------------------------------------

/// Output row geometry: `.master-body` padding and `.master-row` fader layout.
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

/// Verifies that the figure bounding box fits all possible output readings without shifting.
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

/// The knob follows the level, which is the one thing in the row that is meant
/// to move — it sits on the fill's moving edge, and that is the whole of what a
/// fader draws.
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

/// What a run of text lays out to, asked of the same fonts the row was measured
/// against — the one thing in this file that is `egui`'s answer and not the
/// console's.
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

/// Master fader knob clears boundary grab zones at both ends of travel (37.75px vertical clearance).
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

/// The knob is the target and the track is not, which is `Mixer::grab`'s rule
/// and is deliberately not the exposure track's: this fader has a handle drawn
/// on it, and a handle that jumped to the pointer would be a lie about what a
/// handle is.
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

/// Fader travel spans exact `0.0` to `1.0` endpoints and emits slot-independent `SetMasterOut` (ADR-0224).
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

/// When master output level is uninitialized (`View::master_out` is `None`), no row is rendered.
#[test]
fn no_level_behind_the_console_is_no_row_at_all() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    assert!(
        master(&ctx, panel.layout(), None, Some(&mock_chain()), &shut()).is_none(),
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

/// Verifies full pointer lifecycle: claim, in-hand tracking, operation emission, and release.
#[test]
fn the_route_a_window_takes_is_claim_then_derivation_then_operation() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let view = view(MOCK);
    let it = row(&panel, &ctx, MOCK);
    let press = at(it.fader.knob.center());

    assert_eq!(claim(&mut panel, &ctx, &view, press), Claim::Panel);
    let grab = master(
        &ctx,
        panel.layout(),
        view.master_out,
        view.master_chain.as_ref(),
        &shut(),
    )
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
// The master chain's list
// ---------------------------------------------------------------------------

/// One well per slot of the chain, in the chain's order, each naming its
/// procedure and each drawing one parameter row per declared parameter.
#[test]
fn the_bay_draws_one_row_per_slot_of_the_chain() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    let names: Vec<&str> = it.slots.iter().map(|slot| slot.words.as_str()).collect();
    assert_eq!(
        names,
        vec!["feedback", "bloom"],
        "the Master bay drew the chain in an order the engine does not run it in"
    );
    let ats: Vec<u32> = it.slots.iter().map(|slot| slot.at).collect();
    assert_eq!(ats, vec![0, 1], "a slot's row addressed the wrong position");
    let params: Vec<usize> = it.slots.iter().map(|slot| slot.params.len()).collect();
    assert_eq!(params, vec![1, 1]);
    // Chain slots are placed downstream below the out level fader (ADR-0224).
    assert!(
        it.slots[0].well.min.y >= it.fader.track.max.y,
        "a chain row was drawn over the out fader"
    );
    // And `+ add` under the last of them.
    let add = it.add.expect("the bay draws + add at the end of the list");
    assert!(
        add.min.y >= it.slots[1].well.max.y,
        "+ add was drawn over the last slot rather than after it"
    );
}

/// A parameter row's fader is laid out from the range the procedure declares
/// and not from a fixed one, and the operation a drag asks for carries a value
/// in that range.
#[test]
fn a_parameter_rows_fader_rides_the_declared_range() {
    let (mut panel, ctx) = console(PLAUSIBLE);
    let view = view(MOCK);
    let it = row(&panel, &ctx, MOCK);
    let expected: [Operation; 2] = [
        Operation::SetChainParam {
            at: 0,
            param: ChainParam::Declared {
                key: "amount".to_owned(),
                value: 0.95,
            },
        },
        Operation::SetChainParam {
            at: 1,
            param: ChainParam::Declared {
                key: "amount".to_owned(),
                value: 1.0,
            },
        },
    ];
    for (slot, want) in it.slots.iter().zip(expected) {
        let param = &slot.params[0];
        let press = at(param.fader.knob.center());
        assert_eq!(
            claim(&mut panel, &ctx, &view, press),
            Claim::Panel,
            "the panel did not claim a press on slot {}'s knob",
            slot.at
        );
        let grab = master(
            &ctx,
            panel.layout(),
            view.master_out,
            view.master_chain.as_ref(),
            &shut(),
        )
        .and_then(|row| row.grab(press))
        .expect("the derivation that drew the knob says what a press takes hold of");
        panel.grab(press, grab);
        let to = Point::new(param.fader.track.max.x, press.y);
        assert_eq!(
            panel.moved(to),
            Some(Dragged::Fader(want)),
            "slot {}'s track dragged to its far end asked for the wrong thing",
            slot.at
        );
        panel.released(None);
    }
    // The knob's position is a fraction of the declared range and not of the
    // value: 0.34 of 0.95 is not 0.34 of 1.00.
    let feedback = &it.slots[0].params[0];
    assert!(near(feedback.along(), 0.34 / 0.95));
}

/// The cut chip is drawn on a slot whose procedure declares `retains` and on no
/// other.
#[test]
fn only_a_slot_that_retains_carries_a_cut_chip() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    let chips: Vec<bool> = it.slots.iter().map(|slot| slot.cut.is_some()).collect();
    assert_eq!(chips, vec![true, false]);
}

/// A press on the cut chip asks for the other cut on the slot the row
/// addresses, and for nothing else.
#[test]
fn the_cut_chip_asks_for_the_other_cut_on_the_slot_it_names() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    let chip = it.slots[0].cut.expect("the feedback slot draws a cut chip");
    assert_eq!(
        it.chip(at(chip.center())),
        Some(Operation::SetChainParam {
            at: 0,
            param: ChainParam::Cut(Cut::Exit),
        }),
        "the chip on a slot reading the mix cut did not ask for the exit cut on that slot"
    );
    assert_eq!(it.chip(at(it.fader.knob.center())), None);
}

/// The `−` at the end of a slot's row takes that slot out of the chain, by its
/// position.
#[test]
fn the_minus_takes_the_slot_out_by_its_position() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    for slot in &it.slots {
        assert_eq!(
            it.chip(at(slot.remove.center())),
            Some(Operation::RemoveChainEffect { at: slot.at }),
            "the − on slot {} asked for the wrong removal",
            slot.at
        );
    }
}

/// `+ add` puts the chooser down and asks for nothing; picking one of its
/// entries appends a slot of that procedure, with a cut exactly where the
/// procedure declares `retains`.
#[test]
fn the_chooser_adds_the_procedure_that_was_picked() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    let add = it.add.expect("the bay draws + add");
    assert_eq!(it.chose(at(add.center()), &shut()), Some(Added::Open));
    assert!(
        it.card.is_none(),
        "the card was laid out with the chooser up"
    );

    let open = master(
        &ctx,
        panel.layout(),
        Some(MOCK),
        Some(&mock_chain()),
        &down(),
    )
    .expect("the bay draws its body");
    let card = open
        .card
        .expect("the chooser draws its card while it is down");
    assert_eq!(card.items, 2);
    assert_eq!(
        open.chose(at(card.item(0).center()), &down()),
        Some(Added::Add(Operation::AddChainEffect {
            procedure: "sha256:feed".to_owned(),
            cut: Some(Cut::Mix),
        })),
        "a procedure that declares retains was added with no cut"
    );
    assert_eq!(
        open.chose(at(card.item(1).center()), &down()),
        Some(Added::Add(Operation::AddChainEffect {
            procedure: "sha256:b100".to_owned(),
            cut: None,
        })),
        "a procedure that declares no retains was added with one"
    );
    // A press off the card dismisses it and asks for nothing.
    assert_eq!(
        open.chose(at(open.fader.knob.center()), &down()),
        Some(Added::Shut)
    );
}

/// The chain's list is the third set of rectangles a release can land on, and
/// it is one rectangle: an add appends, so every point of the list names the
/// same landing.
#[test]
fn the_chains_list_is_what_a_carried_row_lands_on() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = row(&panel, &ctx, MOCK);
    let list = it.list.expect("the bay draws a list to land on");
    for slot in &it.slots {
        assert_eq!(it.dropped(at(slot.well.center())), Some(list));
    }
    let add = it.add.expect("the bay draws + add");
    assert_eq!(it.dropped(at(add.center())), Some(list));
    // And the out row is not part of it: a level is not a slot.
    assert_eq!(it.dropped(at(it.fader.knob.center())), None);
}

/// A console with a level and no chain draws the out row and nothing under it,
/// which is what a harness that has not written the chain yet gets — and it is
/// not the same as no bay at all.
#[test]
fn no_chain_behind_the_console_is_the_out_row_alone() {
    let (panel, ctx) = console(PLAUSIBLE);
    let it = master(&ctx, panel.layout(), Some(MOCK), None, &shut())
        .expect("the out row draws without a chain behind it");
    assert!(
        it.slots.is_empty(),
        "the Master bay drew chain rows for a console with no chain behind it"
    );
    assert!(
        it.add.is_none() && it.list.is_none(),
        "the Master bay drew + add for a console with no chain behind it"
    );
}
