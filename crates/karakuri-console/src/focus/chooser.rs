use super::*;

/// Handles `space` on a bay to toggle folding across all nine bays (ADR-0333).
///
/// A folded bay responds only to fold toggle so it can be reopened without permitting operations.
pub fn fold(panel: &Panel, bay: &'static Region) -> Asked {
    let Some(id) = panel.layout().find(bay.name) else {
        return Asked::Nothing("this arrangement has no node for the focused bay");
    };
    Asked::Panel(match panel.layout().is_collapsed(id) {
        true => Op::Unfold(id),
        false => Op::Fold(id),
    })
}

/// Returns the number of elements `bay` draws under `path` (`[]`: items, `[n]`: controls, `[n, c]`: sub-controls).
///
/// Returns zero if nothing is drawn, causing actions to decline cleanly.
pub fn drawn(view: &View, built: &Built, path: &[usize]) -> usize {
    match (built.bay, path) {
        (_, []) => match built.items {
            // A headless row's items are its controls, and how many of them
            // are drawn is the bay's own count.
            Items::Controls(of) => match built.bay {
                // **The out fader, one per slot of the chain, and `+ add`** —
                // the bay's own reading, because how many slots there are is
                // what an operator put in the chain. A console with no chain
                // behind it draws the out row and nothing under it.
                MASTER => match view.master_out.is_some() {
                    true => {
                        1 + view
                            .master_chain
                            .as_ref()
                            .map_or(0, |chain| chain.slots.len() + 1)
                    }
                    false => 0,
                },
                OUTPUTS => 1,
                _ => of.len(0),
            },
            Items::Alike(_) => match built.bay {
                MIXER => view.mixer.len(),
                LIBRARY => view.library.len(),
                STAGING => view.staging.len(),
                INSPECTOR => view.inspector.len(),
                SEQUENCER => lanes(view),
                _ => 0,
            },
        },
        // A pane draws one deck head and a group per node.
        (INSPECTOR, [pane]) => {
            1 + view
                .inspector
                .get(pane.wrapping_sub(1))
                .map_or(0, |pane| pane.nodes.len())
        }
        // A node group draws its authority chip, its renderer chips and a row
        // per parameter; the deck head draws its three.
        (INSPECTOR, [pane, through]) => match through {
            1 => 3,
            _ => view
                .inspector
                .get(pane.wrapping_sub(1))
                .and_then(|pane| pane.nodes.get(through.wrapping_sub(2)))
                .map_or(0, |node| 2 + node.params.len()),
        },
        // A lane draws its label and a cell per step of the mode. The minus at
        // the end of the row is not one of them: it is the lane's own act,
        // reached by `enter` on the lane rather than by a digit under it.
        (SEQUENCER, [lane]) if *lane <= lanes(view) => 1 + steps(view),
        // Head card rungs (e.g. Sequencer `+ lane` chooser): draws one row per choice, or zero when closed.
        (_, [HEAD, through]) => match built.head.get(through.wrapping_sub(1)).copied() {
            Some(control) => card_rows(view, control),
            None => 0,
        },
        // Master rungs: slots draw parameters, cut chip, and `−`; `+ add` lists library procedures.
        // Cut chips maintain a consistent digit index across all slots even when omitted (ADR-0352).
        (MASTER, [through]) => match built.item().nth(*through, drawn(view, built, &[])) {
            Some(Control::Slot) => match chain_slot(view, *through) {
                Some(slot) => slot.params.len() + 2,
                None => 0,
            },
            Some(control) => card_rows(view, control),
            None => 0,
        },
        // **The Transport's two cards**: the audio-in card draws one row per
        // input the machine answered with, and the arrangement menu draws
        // *save*, *start a new one* and one row per name filed.
        (TRANSPORT, [through]) => match built.item().nth(*through, built.item().first.len()) {
            Some(control) => card_rows(view, control),
            None => 0,
        },
        (_, [_]) => built.item().first.len(),
        _ => 0,
    }
}

/// Maps Master item index (`through`) to 0-based chain slot index, or `None`.
///
/// Accounts for leading `out` fader offset: slot index is `through - 2`.
fn chain_slot(view: &View, through: usize) -> Option<&crate::view::ChainSlot> {
    view.master_chain
        .as_ref()?
        .slots
        .get(through.checked_sub(2)?)
}

/// The position in the chain the Master bay's `through`th item names.
fn chain_at(through: usize) -> Option<u32> {
    u32::try_from(through.checked_sub(2)?).ok()
}

/// The sentence a digit carries when a slot does not draw that many controls.
pub const SLOT_SHORT: &str =
    "this slot does not draw that many controls — the digits count what is \
                          drawn, from one";

/// Adjusts a chain parameter by 10% of declared range on arrow, or resets to default on `space` (ADR-0343).
pub fn chain_param(view: &View, through: usize, nth: usize, step: Step) -> Asked {
    let (Some(at), Some(slot)) = (chain_at(through), chain_slot(view, through)) else {
        return Asked::Nothing("this slot is not drawn");
    };
    let Some(param) = slot.params.get(nth.wrapping_sub(1)) else {
        return Asked::Nothing("this parameter row is not drawn");
    };
    let [low, high] = param.range;
    let value = match step {
        Step::Default => param.default,
        Step::Up => param.value + (high - low) * PARAM_STEP,
        Step::Down => param.value - (high - low) * PARAM_STEP,
    };
    Asked::Emitted(Operation::SetChainParam {
        at,
        param: ParamOfChain::Declared {
            key: param.key.clone(),
            value: value.clamp(low, high),
        },
    })
}

/// `space` on a slot's cut chip: the other of the two cuts.
pub fn chain_cut(view: &View, through: usize) -> Asked {
    let (Some(at), Some(slot)) = (chain_at(through), chain_slot(view, through)) else {
        return Asked::Nothing("this slot is not drawn");
    };
    let Some(cut) = slot.cut else {
        return Asked::Nothing(
            "this slot's procedure declares no retains, so it reads no retained frame and has \
             no cut to set",
        );
    };
    let all = karakuri_operation::Cut::ALL;
    let showing = all.iter().position(|c| *c == cut).unwrap_or(0);
    Asked::Emitted(Operation::SetChainParam {
        at,
        param: ParamOfChain::Cut(all[(showing + 1) % all.len()]),
    })
}

/// `enter` on a slot's `−`: the slot, taken out of the chain.
pub fn chain_remove(view: &View, through: usize) -> Asked {
    let (Some(at), Some(_)) = (chain_at(through), chain_slot(view, through)) else {
        return Asked::Nothing("this slot is not drawn");
    };
    Asked::Emitted(Operation::RemoveChainEffect { at })
}

/// How many lanes the Sequencer is drawing.
fn lanes(view: &View) -> usize {
    view.sequencer
        .as_ref()
        .map_or(0, |seq| seq.pattern.lanes().len())
}

/// How many cells a lane draws, which is the pattern's mode and not a constant:
/// sixteen at a sixteenth and eight at an eighth (ADR-0306).
fn steps(view: &View) -> usize {
    view.sequencer
        .as_ref()
        .map_or(0, |seq| seq.pattern.mode().count())
}

/// Push `nth` onto the focused bay's address.
pub fn address_into(view: &mut View, bay: &'static str, nth: usize) {
    view.focus_mut().address_mut(bay).down(nth);
}

/// A digit at bay level, naming the nth item — and the deck selection where
/// naming one is that.
pub fn name_item(
    view: &mut View,
    bay: &'static str,
    built: &Built,
    nth: usize,
    items: usize,
) -> Asked {
    if nth > items {
        return Asked::Nothing(match built.selects {
            true => {
                "this deck has no strip with that number — the digits count the strips the \
                     mixer drew, from one"
            }
            false => {
                "this bay is not drawing that many things — the digits count what is drawn, \
                      from one"
            }
        });
    }
    match (built.selects, built.bay) {
        (true, _) => {
            view.select((nth - 1) as u8);
        }
        (false, LIBRARY) => {
            view.point_at(nth - 1);
        }
        _ => {}
    }
    // **Refused rather than clamped, and the address does not descend on a
    // refusal** — the guard above is that refusal, and it is the bay's own
    // count rather than a second rule.
    address_into(view, bay, nth);
    match built.selects {
        true => Asked::Emitted(Operation::SelectDeck {
            deck: (nth - 1) as u8,
        }),
        false => Asked::Moved,
    }
}

/// Walks bay items via arrows using the remembered item if no item is addressed (ADR-0333).
pub fn walk(
    view: &mut View,
    panel: &Panel,
    bay: &'static str,
    built: &Built,
    arrow: Arrow,
    items: usize,
    at: &[usize],
) -> Asked {
    if items == 0 {
        return Asked::Nothing("this bay is drawing nothing to walk");
    }
    let step = arrow.step();
    let moved = match (built.selects, built.bay) {
        // **The strips are walked and not wrapped**, which is `View::walk`'s
        // rule one bay over: a walk is not a cycle, and a press held down must
        // not jump the length of the row.
        (true, _) => {
            let from = i64::from(view.selection());
            let to = (from + i64::from(step)).clamp(0, items as i64 - 1) as u8;
            view.select(to)
        }
        (false, LIBRARY) => {
            let showing = crate::view::library(
                panel.layout(),
                &view.scopes,
                &view.library,
                view.opened(),
                view.pointed(),
                view.library_scroll(),
            )
            .map_or(0..0, |bay| bay.drawn());
            view.walk(step, showing)
        }
        // **Every other bay walks the address alone**, because the item it is
        // on is not a pointer anything downstream reads: the deck selection
        // and the library cursor are the two that are, and each is a bay's
        // remembered address seen from outside (ADR-0332).
        _ => {
            let from = remembered(view, bay, items);
            let to = (from as i64 + i64::from(step)).clamp(1, items as i64) as usize;
            to != from
        }
    };
    // The address follows the walk where it had descended to an item, and stays
    // at the bay where it had not — and the memory follows it either way, which
    // is what the solid ring is.
    let nth = match (built.selects, built.bay) {
        (true, _) => usize::from(view.selection()) + 1,
        (false, LIBRARY) => view.cursor_row() + 1,
        _ => {
            (remembered(view, bay, items) as i64 + i64::from(step)).clamp(1, items as i64) as usize
        }
    };
    match at.is_empty() {
        true => {
            view.focus_mut().address_mut(bay).remember(&[], nth);
        }
        false => view.focus_mut().address_mut(bay).to_item(nth),
    }
    match (built.selects, moved) {
        // **Emitted whether or not the mark moved**, which is the four deck
        // keys' rule this replaces: what a press asked for is what is emitted.
        (true, _) => Asked::Emitted(Operation::SelectDeck {
            deck: view.selection(),
        }),
        (false, true) => Asked::Moved,
        (false, false) => Asked::Nothing("the cursor is already at the end of what is drawn"),
    }
}

/// Which item a bay is remembering, one-based and inside what it is drawing —
/// the solid ring, read where the walk needs somewhere to start.
pub(crate) fn remembered(view: &View, bay: &str, items: usize) -> usize {
    view.focus()
        .address(bay)
        .and_then(|address| address.remembered(&[]))
        .unwrap_or(1)
        .clamp(1, items.max(1))
}

/// An arrow on a control: the next value of a level, or the neighbouring cell
/// of a row of them.
#[allow(clippy::too_many_arguments)]
pub fn stepped(view: &View, item: Option<usize>, control: Control, arrow: Arrow) -> Asked {
    match control.answers() {
        // A track is stepped exactly as a level is; what it has not got is a
        // value `space` returns it to.
        Answers::Level | Answers::Track(_) => match arrow {
            Arrow::Up | Arrow::Down => {
                let step = match arrow {
                    Arrow::Up => Step::Up,
                    _ => Step::Down,
                };
                level(view, item, control, step)
            }
            _ => Asked::Nothing("a level is stepped up and down, not across"),
        },
        Answers::Cells { across } => match arrow.across() == across {
            true => Asked::Nothing(
                "the cells of a lane are walked from the cell the address is on — press a digit \
                 to name one first",
            ),
            false => Asked::Nothing("a lane's cells are a row, so left and right walk them"),
        },
        _ => Asked::Nothing(
            "this control's values are a closed list and a list has no axis — space cycles it",
        ),
    }
}

/// An arrow on the third rung: the Inspector's levels, and nothing else.
#[allow(clippy::too_many_arguments)]
pub fn under_arrow(
    view: &View,
    item: usize,
    through: usize,
    nth: usize,
    control: Control,
    arrow: Arrow,
) -> Asked {
    if !matches!(arrow, Arrow::Up | Arrow::Down) {
        return Asked::Nothing("a level is stepped up and down, not across");
    }
    let up = matches!(arrow, Arrow::Up);
    match control {
        // **A quarter beat either way**, which is the deck head's own arrows.
        Control::Anchor => match view.inspector.get(item.wrapping_sub(1)) {
            Some(pane) => Asked::Emitted(Operation::ScrubDeck {
                deck: pane.deck as u8,
                beats: match up {
                    true => crate::view::SCRUB_BEATS,
                    false => -crate::view::SCRUB_BEATS,
                },
            }),
            None => Asked::Nothing("this pane is not drawn"),
        },
        // **A tenth of the published range**, which is the trim's tenth read on
        // a control whose range is declared rather than fixed — and the value
        // it steps from is the pane's, for `view::ParamGrip`'s reason.
        Control::Param => param(view, item, through, nth, up),
        _ => Asked::Nothing(
            "this control's values are a closed list and a list has no axis — space cycles it",
        ),
    }
}

/// A parameter row, stepped a tenth of what it publishes.
fn param(view: &View, item: usize, through: usize, nth: usize, up: bool) -> Asked {
    let Some(param) = view
        .inspector
        .get(item.wrapping_sub(1))
        .and_then(|pane| pane.nodes.get(through.wrapping_sub(2)))
        .and_then(|node| node.params.get(nth.wrapping_sub(3)))
    else {
        return Asked::Nothing("this parameter row is not drawn");
    };
    let deck = match view.inspector.get(item.wrapping_sub(1)) {
        Some(pane) => pane.deck as u8,
        None => return Asked::Nothing("this pane is not drawn"),
    };
    let to = param.at()
        + match up {
            true => PARAM_STEP,
            false => -PARAM_STEP,
        };
    Asked::Emitted(Operation::WriteParam {
        deck,
        param: param.param.clone(),
        value: ParamValue::Scalar(param.valued(to)),
    })
}

/// One press of a parameter key, as a fraction of what the control publishes —
/// the trim's tenth read on a range that is declared rather than fixed.
/// `Param::valued` clamps it, which is that method's own rule.
const PARAM_STEP: f32 = 0.1;

/// A level the host steps, named here and stepped there.
fn level(view: &View, item: Option<usize>, control: Control, step: Step) -> Asked {
    match control {
        Control::Trim => Asked::Stepped {
            level: Level::Trim((item.unwrap_or(1) - 1) as u8),
            step,
        },
        Control::Fader => Asked::Stepped {
            level: Level::Fader((item.unwrap_or(1) - 1) as u8),
            step,
        },
        Control::Out => match view.master_out.is_some() {
            true => Asked::Stepped {
                level: Level::Out,
                step,
            },
            false => Asked::Nothing("this console has no master out behind it to step"),
        },
        // **The tempo the host steps is the one the grid is running**, read
        // off the oscillator at the press rather than off the figure a frame
        // drew — which is the three mix keys' own rule (ADR-0333). What the
        // step is, is `karakuri/src/bridge/handlers.rs`'s `tempo_key`.
        Control::Tempo => match view.transport.is_some() {
            true => Asked::Stepped {
                level: Level::Tempo,
                step,
            },
            false => {
                Asked::Nothing("this console has no engine behind it, so there is no grid to step")
            }
        },
        Control::Exposure => match view.look.is_some() {
            true => Asked::Stepped {
                level: Level::Exposure,
                step,
            },
            false => Asked::Nothing("this console has no look behind it, so there is no exposure"),
        },
        Control::Offset => match view.tracker.is_some() {
            true => Asked::Stepped {
                level: Level::Offset,
                step,
            },
            false => Asked::Nothing(
                "no audio session is open, so there is no latency offset to nudge — attach an \
                 input on the audio-in pill first",
            ),
        },
        _ => Asked::Nothing("this control is not a level"),
    }
}

/// `space` on a control — the next state, named here because the cycle is this
/// console's affordance (P-0090) and read off the deck by the caller where the
/// reading is not.
#[allow(clippy::too_many_arguments)]
pub fn cycled(
    view: &View,
    panel: &Panel,
    item: Option<usize>,
    nth: usize,
    control: Control,
    held: impl FnOnce(u8) -> Option<Held>,
) -> Asked {
    match control.answers() {
        Answers::Level => level(view, item, control, Step::Default),
        Answers::Nothing(why) | Answers::Track(why) => Asked::Nothing(why),
        Answers::Act(_) => Asked::Nothing(
            "this control performs rather than sets, so it has no next state — enter runs it",
        ),
        Answers::State | Answers::Cells { .. } => state(view, panel, item, nth, control, held),
    }
}

/// The next state of a control whose values are a closed list, and the
/// operation that names where it arrived.
#[allow(clippy::too_many_arguments)]
fn state(
    view: &View,
    panel: &Panel,
    item: Option<usize>,
    nth: usize,
    control: Control,
    held: impl FnOnce(u8) -> Option<Held>,
) -> Asked {
    match control {
        // -- the Mixer's five --------------------------------------------
        Control::Tally | Control::Blend | Control::Mask => {
            let deck = (item.unwrap_or(1) - 1) as u8;
            let Some(held) = held(deck) else {
                return Asked::Nothing(
                    "this deck has no slot behind that strip, so there is nothing to read a \
                     state off",
                );
            };
            Asked::Emitted(match control {
                Control::Tally => Operation::SetResidency {
                    deck,
                    residency: crate::view::residency(crate::view::next(held.requested)),
                },
                Control::Blend => Operation::SetBlendMode {
                    deck,
                    blend: crate::view::after(held.blend),
                },
                _ => Operation::SetMaskShape {
                    deck,
                    kind: crate::view::wipe_kind(crate::view::next_shape(held.mask)),
                    angle: held.mask_angle,
                },
            })
        }
        // -- the Mixer's head, which is the transition row -----------------
        Control::Shape => Asked::Emitted(Operation::SetTransition {
            setting: view.transition().next_wipe_shape(),
        }),
        Control::Quantum => Asked::Emitted(Operation::SetTransition {
            setting: view.transition().next_quantum(),
        }),
        Control::Length => Asked::Emitted(Operation::SetTransition {
            setting: view.transition().next_length(),
        }),
        // -- the Library ---------------------------------------------------
        Control::Scope => Asked::Scope,
        Control::Star => match view.rows().set(item.unwrap_or(1) - 1) {
            Some(id) => Asked::Emitted(Operation::SetFavourite {
                id: id.to_owned(),
                favourite: !view.starred.contains(id),
            }),
            None => Asked::Nothing(
                "this row is not a Set, so there is nothing to star — a row of history is a \
                 version",
            ),
        },
        // -- the Transport -------------------------------------------------
        Control::Tonemap => match view.look {
            Some(look) => Asked::Emitted(Operation::SetTonemap {
                tonemap: crate::view::next_tonemap(look.tonemap),
            }),
            None => Asked::Nothing("this console has no look behind it, so there is no tone map"),
        },
        // -- the Program bay -----------------------------------------------
        Control::Solo => match panel.layout().is_soloed() {
            true => Asked::Panel(Op::Unsolo),
            false => match panel.layout().find(PICTURE) {
                Some(id) => Asked::Panel(Op::Solo(id)),
                None => Asked::Nothing("this arrangement draws no picture to solo"),
            },
        },
        Control::Class => Asked::Nothing(
            "the class pill is opened by pressing it, and what it opens is a class rather than \
             a state to cycle — press it",
        ),
        // -- the Inspector -------------------------------------------------
        Control::Sync | Control::Composite | Control::Authority | Control::Renderer => {
            Asked::Nothing("this control is reached under a pane, not here")
        }
        // -- the Sequencer -------------------------------------------------
        Control::Mode => match view.sequencer.as_ref() {
            Some(seq) => Asked::Emitted(Operation::SetPatternGrid {
                pattern: seq.bank as u8,
                grid: match seq.pattern.mode() {
                    StepMode::Sixteenth => StepMode::Eighth,
                    StepMode::Eighth => StepMode::Sixteenth,
                },
            }),
            None => Asked::Nothing("this console has no pattern behind it"),
        },
        Control::Bank => Asked::Emitted(Operation::SelectPattern {
            // The head draws the mode pill and then the four banks, so the
            // bank a digit named is the one it counted to less that pill.
            pattern: (nth - 2) as u8,
        }),
        Control::Label => lane_state(view, item.unwrap_or(1)),
        Control::Step => step_state(view, item.unwrap_or(1), nth),
        // -- the Outputs row -----------------------------------------------
        Control::Sink => sink(panel),
        _ => Asked::Nothing("nothing here answers space"),
    }
}

/// `space` on a lane's label: the mute, named as the state it arrives at.
fn lane_state(view: &View, lane: usize) -> Asked {
    let Some(seq) = view.sequencer.as_ref() else {
        return Asked::Nothing("this console has no pattern behind it");
    };
    match seq.pattern.lanes().get(lane.wrapping_sub(1)) {
        Some(found) => Asked::Emitted(Operation::SetLaneMute {
            pattern: seq.bank as u8,
            lane: (lane - 1) as u8,
            muted: !found.muted(),
        }),
        None => Asked::Nothing("this pattern has no lane with that number"),
    }
}

/// `enter` on a lane: the lane, taken out of the pattern by the position it is
/// drawn at — which is what the minus at the end of its row asks for. The lanes
/// after it move up, which is what a lane index means
/// (`karakuri_pattern::Pattern::remove`).
fn lane_removed(view: &View, lane: usize) -> Asked {
    let Some(seq) = view.sequencer.as_ref() else {
        return Asked::Nothing("this console has no pattern behind it");
    };
    match seq.pattern.lanes().get(lane.wrapping_sub(1)) {
        Some(_) => Asked::Emitted(Operation::RemoveLane {
            pattern: seq.bank as u8,
            lane: (lane - 1) as u8,
        }),
        None => Asked::Nothing("this pattern has no lane with that number"),
    }
}

/// `space` on a cell: the step, named as the state it arrives at — and the
/// stored slot rather than the drawn step, which is what keeps the payload
/// independent of the mode.
fn step_state(view: &View, lane: usize, nth: usize) -> Asked {
    let Some(seq) = view.sequencer.as_ref() else {
        return Asked::Nothing("this console has no pattern behind it");
    };
    let Some(found) = seq.pattern.lanes().get(lane.wrapping_sub(1)) else {
        return Asked::Nothing("this pattern has no lane with that number");
    };
    // The lane draws its label and then its cells, so the cell a digit named is
    // the one it counted to less that label.
    let step = nth.wrapping_sub(2);
    let mode = seq.pattern.mode();
    if step >= mode.count() {
        return Asked::Nothing("this lane is not drawing a cell with that number");
    }
    let slot = mode.slot_of(step);
    Asked::Emitted(Operation::SetStep {
        pattern: seq.bank as u8,
        lane: (lane - 1) as u8,
        step: slot as u8,
        on: !found.slot_on(slot),
    })
}

/// `space` on a sink: the picture's on and off, which is one press asking for
/// the operation that names the output and the fold that carries it out.
fn sink(panel: &Panel) -> Asked {
    let Some(id) = panel.layout().find(PICTURE) else {
        return Asked::Nothing("this arrangement draws no picture");
    };
    let on = panel.layout().visible(id);
    Asked::Routed(
        Operation::RouteFrame {
            output: Output::Program,
            on: !on,
        },
        match on {
            true => Op::Fold(id),
            false => Op::Unfold(id),
        },
    )
}

/// `space` on the third rung — the Inspector's chips, each naming the state it
/// arrives at rather than a flip, which is P-0090 and the chips' own rule.
pub fn under_space(
    view: &View,
    item: usize,
    through: usize,
    nth: usize,
    control: Control,
) -> Asked {
    let Some(pane) = view.inspector.get(item.wrapping_sub(1)) else {
        return Asked::Nothing("this pane is not drawn");
    };
    let deck = pane.deck as u8;
    match control {
        Control::Sync => Asked::Emitted(Operation::SetSync {
            deck,
            sync: crate::view::next_sync(pane.sync, pane.allows),
        }),
        Control::Composite => Asked::Emitted(Operation::SetCompositing {
            deck,
            compositing: !pane.composite,
        }),
        Control::Authority => match pane
            .nodes
            .get(through.wrapping_sub(2))
            .and_then(|node| node.authority)
        {
            Some(authority) => Asked::Emitted(Operation::SetAuthority {
                deck,
                node: authority.at,
                authority: next_authority(authority.level),
            }),
            None => Asked::Nothing(
                "this head stands over more than one node, so there is no one authority to set \
                 — open the fold and name the node",
            ),
        },
        Control::Renderer => match pane.nodes.get(through.wrapping_sub(2)) {
            Some(node) if !node.renderers.is_empty() && pane.composite => {
                let live = node.renderers.iter().position(|r| r.live).unwrap_or(0);
                Asked::Emitted(Operation::SelectRenderer {
                    deck,
                    renderer: ((live + 1) % node.renderers.len()) as u32,
                })
            }
            Some(_) => Asked::Nothing(
                "this deck overdraws its renderers, so they all draw and none of them is live \
                 — the composite chip in the deck head is what makes it a choice",
            ),
            None => Asked::Nothing("this pane is not drawing that node"),
        },
        Control::Anchor | Control::Param => {
            let _ = nth;
            Asked::Nothing(
                "this control is a level, and a level has no next state — the arrows step it",
            )
        }
        _ => Asked::Nothing("nothing here answers space"),
    }
}

/// The next of the three authorities, wrapping — the cycle a chip row is,
/// curated here because the list is the vocabulary's and the cycle is the
/// surface's (`view::AUTHORITIES`' own argument, one control along).
fn next_authority(level: Authority) -> Authority {
    let at = crate::view::AUTHORITIES
        .iter()
        .position(|found| *found == level)
        .unwrap_or(0);
    crate::view::AUTHORITIES[(at + 1) % crate::view::AUTHORITIES.len()]
}

/// `enter` on a control, where the control is an act.
pub fn act_of(view: &View, item: Option<usize>, control: Control) -> Asked {
    match control.answers() {
        Answers::Act(act) => performed(view, item, act),
        Answers::Nothing(why) => Asked::Nothing(why),
        _ => Asked::Nothing(
            "nothing here performs — enter is the act the addressed control is for, and this one \
             sets rather than performs",
        ),
    }
}

/// What `enter` performs, per act.
pub fn performed(view: &View, item: Option<usize>, act: Act) -> Asked {
    match act {
        Act::Load => Asked::Load,
        // **The two chain acts arrive on the rung under a control**, where
        // this function is reached from an *item*, so neither is asked here.
        // The arms are `Addressed::Of`'s and `Addressed::InCard`'s in
        // [`press`], which have the slot's position and the card's entry.
        Act::Remove | Act::Add => Asked::Nothing(
            "this is not the rung this act is on — press a digit to name a slot of the chain",
        ),
        // **A lane is an item and taking it out is the item's own act**, so
        // unlike the two chain acts above this one *is* reached here, with the
        // lane's own number in hand.
        Act::RemoveLane => lane_removed(view, item.unwrap_or(1)),
        Act::Read => match view.rows().set(item.unwrap_or(1) - 1) {
            Some(id) => Asked::Emitted(Operation::ReadSet { id: id.to_owned() }),
            None => Asked::Nothing(
                "this row is not a Set, so there is nothing to read — a row of history is a \
                 version",
            ),
        },
        Act::Keep => match view.staging.get(item.unwrap_or(1) - 1) {
            Some(candidate) => match (candidate.at, candidate.stage) {
                (Some(node), stage) if stage != crate::view::Stage::Overloaded => {
                    Asked::Emitted(Operation::KeepCandidate {
                        deck: candidate.deck as u8,
                        node,
                    })
                }
                (Some(_), _) => Asked::Nothing(
                    "this candidate cost more than the frame allows, so there is nothing to keep \
                     — the slot is stopped on the version it last drew",
                ),
                (None, _) => Asked::Nothing("this row names no node, so there is nothing to keep"),
            },
            None => Asked::Nothing("this lane is not drawing that many rows"),
        },
        Act::Back => match view.staging.get(item.unwrap_or(1) - 1) {
            Some(candidate) => match candidate.at {
                Some(node) => Asked::Emitted(Operation::RestoreProcedure {
                    deck: candidate.deck as u8,
                    revision: Revision::Previous(node),
                }),
                None => Asked::Nothing(
                    "this row names no node, so there is no previous version to put back",
                ),
            },
            None => Asked::Nothing("this lane is not drawing that many rows"),
        },
        // **Taking a parameter back is the third rung's**, because it needs
        // the pane, the node and the row the address walked through — see
        // [`under_enter`], which is the only caller that has all three.
        Act::TakeBack => Asked::Nothing(
            "a parameter is taken back on the row that draws it — press a digit to name the \
             pane, the node and the row",
        ),
        // **A card and its rows are reached on the control the card hangs
        // from**, because both have the address to move and the card to put
        // down or take away — see [`open_card`] and [`card_enter`], which
        // [`press`] reaches before this function.
        Act::Open | Act::Point => Asked::Nothing(
            "the chooser is reached in the Sequencer's head — press 0 and then the digit that \
             names + lane",
        ),
        // **The Transport's two cards are reached on the row that draws
        // them**, because each needs the pill the address descended through
        // and the card to take away — see [`card_enter`], which [`press`]
        // reaches before this function.
        Act::Attach | Act::Save | Act::Restore => Asked::Nothing(
            "this is a row of a card in the Transport — press enter on the pill that puts the \
             card down, then a digit to name the row",
        ),
        // **The addressed strip is covered and the next one round arrives over
        // it**, which is `karakuri-cli`'s `c` and the `go` capsule's own
        // reading of *the next deck*.
        Act::Go => {
            let decks = view.mixer.len();
            if decks < 2 {
                return Asked::Nothing(
                    "a transition needs somewhere to come from — this mixer draws one strip",
                );
            }
            if !view.transition().armed() {
                return Asked::Nothing(
                    "no shape is chosen, so there is nothing for the front to be — press space \
                     on the shape pill in this bay's head",
                );
            }
            let from = usize::from(view.selection()).min(decks - 1);
            Asked::Emitted(Operation::Wipe {
                from: from as u8,
                to: ((from + 1) % decks) as u8,
            })
        }
    }
}

/// `enter` on the third rung: taking a parameter back, which is the one act the
/// Inspector draws that the address reaches.
pub fn under_enter(
    view: &View,
    item: usize,
    through: usize,
    nth: usize,
    control: Control,
) -> Asked {
    if control != Control::Param {
        return Asked::Nothing(
            "nothing here performs — enter is the act the addressed control is for, and this one \
             sets rather than performs",
        );
    }
    let Some(pane) = view.inspector.get(item.wrapping_sub(1)) else {
        return Asked::Nothing("this pane is not drawn");
    };
    let Some(param) = pane
        .nodes
        .get(through.wrapping_sub(2))
        .and_then(|node| node.params.get(nth.wrapping_sub(3)))
    else {
        return Asked::Nothing("this parameter row is not drawn");
    };
    match param.bound.as_ref() {
        Some(source) => Asked::Emitted(Operation::TakeParamBack {
            deck: pane.deck as u8,
            param: source.at.clone(),
        }),
        None => Asked::Nothing(
            "nothing is holding this control, so there is nothing to take back — the arrows \
             write it",
        ),
    }
}
