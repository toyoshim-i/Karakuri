//! Focus ring traversal and keyboard navigation grammar (ADR-0259, ADR-0332, ADR-0333, ADR-0343).
//!
//! Traverses arrangement bays (`Tab`/`shift-Tab`) using hierarchical address paths and keyboard grammar.

use std::collections::BTreeMap;

use karakuri_layout::{Layout, NodeId, Point};
use karakuri_operation::{
    Authority, BeatSource, BlendMode, ChainParam as ParamOfChain, Operation, Output, ParamValue,
    Revision, StepMode,
};

use crate::panel::{Op, Panel};
use crate::view::{region, Ask, AudioAsk, Kind, Mask, Region, Tally, View};

pub mod built;
pub mod card;
pub mod chooser;
pub mod ladder;
pub mod model;

pub use built::*;
pub use card::*;
pub use chooser::*;
pub use ladder::*;
pub use model::*;

/// The sentence for a key pressed in a bay whose grammar is not built. No bay
/// is, as of ADR-0343, and the arm stays because a tenth region added to the
/// panel would arrive here rather than at a panic.
const NOT_BUILT: &str = "the six keys are not built in this bay yet — space still folds it, and \
                         tab moves on to the next";

/// Handles keyboard grammar navigation and actions within the focused bay.
///
/// Resolves the address against current layout and view state, dispatching to
/// appropriate view selection or step methods (ADR-0259).
pub fn press(
    view: &mut View,
    panel: &Panel,
    key: Press,
    held: impl FnOnce(u8) -> Option<Held>,
) -> Asked {
    let Some(bay) = view.focused(panel) else {
        return Asked::Nothing("this arrangement draws no bay to address a key to");
    };
    // A folded bay accepts only `space` (to unfold) and rejects all inner navigation (ADR-0259).
    if panel
        .layout()
        .find(bay.name)
        .is_some_and(|id| !panel.layout().visible(id))
    {
        return match key {
            Press::Space => fold(panel, bay),
            _ => Asked::Nothing(
                "this bay is folded away, so there is nothing drawn for this key to land on — \
                 space opens it, and tab moves on",
            ),
        };
    }
    // Space at bay level toggles folding across all bays (ADR-0259, ADR-0343).
    if key == Press::Space
        && view
            .focus()
            .address(bay.name)
            .is_none_or(|address| address.at().is_empty())
    {
        return fold(panel, bay);
    }
    let Some(built) = built(bay.name) else {
        return Asked::Nothing(NOT_BUILT);
    };
    let at: Vec<usize> = view
        .focus()
        .address(bay.name)
        .map_or_else(Vec::new, |address| address.at().to_vec());
    let items = drawn(view, built, &[]);
    let Some(here) = addressed(built, &at, |path| drawn(view, built, path)) else {
        // Reset focus to bay level if previously focused target is no longer drawn.
        view.focus_mut().address_mut(bay.name).to_the_bay();
        return Asked::Nothing(
            "the address was on something this bay has stopped drawing, so it is back at the \
             bay — press a digit to name what is there now",
        );
    };
    match (here, key) {
        // ------------------------------------------------------------------
        // A digit — the nth thing one level below the address, `0` the head
        // ------------------------------------------------------------------
        (Addressed::Bay, Press::Digit(HEAD)) => match built.head.is_empty() {
            true => Asked::Nothing(
                "this bay draws no head, so 0 names the row itself — press 1 for the first of \
                 the controls in it",
            ),
            false => {
                address_into(view, bay.name, HEAD);
                Asked::Moved
            }
        },
        (Addressed::Bay, Press::Digit(nth)) => name_item(view, bay.name, built, nth, items),
        (Addressed::Head, Press::Digit(nth)) => match built.head.get(nth.wrapping_sub(1)) {
            Some(_) => {
                address_into(view, bay.name, nth);
                Asked::Moved
            }
            None => Asked::Nothing(
                "this bay's head does not draw that many controls — the digits count what is \
                 drawn, from one",
            ),
        },
        (Addressed::Item(item), Press::Digit(nth)) => {
            match built.item().nth(nth, drawn(view, built, &[item])) {
                Some(_) => {
                    address_into(view, bay.name, nth);
                    Asked::Moved
                }
                None => Asked::Nothing(
                    "this item does not draw that many controls — the digits count what is \
                     drawn, from one",
                ),
            }
        }
        // Digits descend into sub-controls (Inspector deck head or node group).
        (
            Addressed::Of {
                item: Some(item),
                nth: through,
                control,
            },
            Press::Digit(nth),
        ) if control.reached_by(Grammar::Digit) => {
            match built
                .beneath(control)
                .nth(nth, drawn(view, built, &[item, through]))
            {
                Some(_) => {
                    address_into(view, bay.name, nth);
                    Asked::Moved
                }
                None => Asked::Nothing(
                    "this does not draw that many controls — the digits count what is drawn, \
                     from one",
                ),
            }
        }
        // Digits navigate into Master chain slot controls.
        (
            Addressed::Of {
                item: None,
                nth: through,
                control: Control::Slot,
            },
            Press::Digit(nth),
        ) => match built
            .beneath(Control::Slot)
            .nth(nth, drawn(view, built, &[through]))
        {
            Some(_) => {
                address_into(view, bay.name, nth);
                Asked::Moved
            }
            None => Asked::Nothing(SLOT_SHORT),
        },
        // Digits select rows within open modal cards.
        (Addressed::OfHead(through, control), Press::Digit(nth)) => {
            name_row(view, bay.name, built, control, &[HEAD, through], nth)
        }
        (
            Addressed::Of {
                item: None,
                nth: through,
                control,
            },
            Press::Digit(nth),
        ) => name_row(view, bay.name, built, control, &[through], nth),
        (
            Addressed::Of { .. }
            | Addressed::Under { .. }
            | Addressed::UnderHead { .. }
            | Addressed::InCard { .. },
            Press::Digit(_),
        ) => Asked::Nothing(NOTHING_BELOW),

        // ------------------------------------------------------------------
        // The arrows — the neighbour, or the next value
        // ------------------------------------------------------------------
        (Addressed::Bay | Addressed::Item(_), Press::Arrow(arrow)) => {
            match arrow.across() == built.across {
                true => walk(view, panel, bay.name, built, arrow, items, &at),
                false => Asked::Nothing(match built.across {
                    true => "this bay's items are a row, so left and right walk them",
                    false => "this bay's items are a column, so up and down walk them",
                }),
            }
        }
        // Steps chain parameter levels by 0.1 of procedure range.
        (
            Addressed::InCard {
                through,
                nth,
                control: Control::ChainParam,
            },
            Press::Arrow(arrow),
        ) => match arrow {
            Arrow::Up => chain_param(view, through, nth, Step::Up),
            Arrow::Down => chain_param(view, through, nth, Step::Down),
            _ => Asked::Nothing("a level is stepped up and down, not across"),
        },
        (
            Addressed::InCard {
                control: Control::ChainCut | Control::ChainRemove,
                ..
            },
            Press::Arrow(_),
        ) => Asked::Nothing(
            "this control's values are a closed list and a list has no axis — space cycles it",
        ),
        // Walks card rows using remembered or initial selection.
        (Addressed::OfHead(through, control), Press::Arrow(arrow))
            if card_of(control).is_some() =>
        {
            walk_card(
                view,
                bay.name,
                built,
                control,
                &[HEAD, through],
                None,
                arrow,
            )
        }
        (Addressed::UnderHead { through, nth, .. }, Press::Arrow(arrow)) => {
            match built.head.get(through.wrapping_sub(1)).copied() {
                Some(above) if card_of(above).is_some() => walk_card(
                    view,
                    bay.name,
                    built,
                    above,
                    &[HEAD, through],
                    Some(nth),
                    arrow,
                ),
                _ => Asked::Nothing(
                    "this control's values are a closed list and a list has no axis — space \
                     cycles it",
                ),
            }
        }
        (
            Addressed::Of {
                item: None,
                nth: through,
                control,
            },
            Press::Arrow(arrow),
        ) if card_of(control).is_some() => {
            walk_card(view, bay.name, built, control, &[through], None, arrow)
        }
        (Addressed::InCard { through, nth, .. }, Press::Arrow(arrow)) => {
            match built.item().nth(through, items) {
                Some(above) => {
                    walk_card(view, bay.name, built, above, &[through], Some(nth), arrow)
                }
                None => Asked::Nothing("this control is not drawn"),
            }
        }
        (Addressed::OfHead(_, control), Press::Arrow(arrow)) => stepped(view, None, control, arrow),
        (
            Addressed::Of {
                item,
                nth: _,
                control,
            },
            Press::Arrow(arrow),
        ) => stepped(view, item, control, arrow),
        (
            Addressed::Under {
                item,
                through,
                nth,
                control,
            },
            Press::Arrow(arrow),
        ) => under_arrow(view, item, through, nth, control, arrow),
        // Head controls are addressed by digit rather than arrow navigation.
        (Addressed::Head, Press::Arrow(_)) => Asked::Nothing(
            "a head's controls are named by a digit rather than walked — press 1 for the first \
             of them",
        ),

        // ------------------------------------------------------------------
        // `space` — the addressed thing's next state
        // ------------------------------------------------------------------
        (Addressed::Head, Press::Space) => Asked::Nothing(
            "a head is not a control — press a digit to name one of the controls in it",
        ),
        (Addressed::Item(_), Press::Space) => Asked::Nothing(
            "this item has no state of its own — press a digit to name one of the controls in \
             it, where it draws any",
        ),
        (Addressed::OfHead(nth, control), Press::Space) => {
            cycled(view, panel, None, nth, control, held)
        }
        (Addressed::Of { item, nth, control }, Press::Space) => {
            cycled(view, panel, item, nth, control, held)
        }
        // Chain slot controls: cycles cut chip or resets parameter to default.
        (
            Addressed::InCard {
                through,
                control: Control::ChainCut,
                ..
            },
            Press::Space,
        ) => chain_cut(view, through),
        (
            Addressed::InCard {
                through,
                nth,
                control: Control::ChainParam,
            },
            Press::Space,
        ) => chain_param(view, through, nth, Step::Default),
        // Card rows perform actions via `enter` rather than cycling state.
        (Addressed::InCard { control, .. }, Press::Space) => match control.answers() {
            Answers::Nothing(why) => Asked::Nothing(why),
            _ => Asked::Nothing(
                "this row performs rather than sets, so it has no next state — enter runs it",
            ),
        },
        (
            Addressed::Under {
                item,
                through,
                nth,
                control,
            },
            Press::Space,
        ) => under_space(view, item, through, nth, control),
        // Chooser entries perform actions rather than holding state.
        (Addressed::UnderHead { .. }, Press::Space) => Asked::Nothing(
            "this control performs rather than sets, so it has no next state — enter runs it",
        ),
        (Addressed::Bay, Press::Space) => fold(panel, bay),

        // ------------------------------------------------------------------
        // `enter` — the act the addressed thing is for
        // ------------------------------------------------------------------
        (Addressed::Item(item), Press::Enter) => match built.act {
            Some(act) => performed(view, Some(item), act),
            None => Asked::Nothing(
                "this item performs nothing — enter is the act a control is for, and this bay's \
                 items are things you set rather than things you run",
            ),
        },
        // Opens card from its parent control.
        (Addressed::OfHead(_, control), Press::Enter) if card_of(control).is_some() => {
            open_card(view, control)
        }
        // Executes addressed card row action and closes the card.
        (
            Addressed::UnderHead {
                through,
                nth,
                control,
            },
            Press::Enter,
        ) => match built.head.get(through.wrapping_sub(1)).copied() {
            Some(above) if card_of(above).is_some() => {
                card_enter(view, bay.name, built, above, control, nth)
            }
            _ => Asked::Nothing(
                "nothing here performs — enter is the act the addressed control is for, \
                     and this one sets rather than performs",
            ),
        },
        (Addressed::OfHead(_, control), Press::Enter) => act_of(view, None, control),
        // Enter on slot remove button removes it from the chain.
        (
            Addressed::InCard {
                through,
                control: Control::ChainRemove,
                ..
            },
            Press::Enter,
        ) => chain_remove(view, through),
        (
            Addressed::Of {
                item: None,
                control,
                ..
            },
            Press::Enter,
        ) if card_of(control).is_some() => open_card(view, control),
        (Addressed::Of { item, control, .. }, Press::Enter) => act_of(view, item, control),
        (
            Addressed::InCard {
                through,
                nth,
                control,
            },
            Press::Enter,
        ) => match built.item().nth(through, items) {
            Some(above) => card_enter(view, bay.name, built, above, control, nth),
            None => Asked::Nothing("this control is not drawn"),
        },
        (
            Addressed::Under {
                item,
                through,
                nth,
                control,
            },
            Press::Enter,
        ) => under_enter(view, item, through, nth, control),
        (Addressed::Bay, Press::Enter) => {
            if items > 0 {
                let nth = remembered(view, bay.name, items);
                name_item(view, bay.name, built, nth, items)
            } else if !built.head.is_empty() {
                address_into(view, bay.name, HEAD);
                Asked::Moved
            } else {
                Asked::Nothing(
                    "nothing here performs — enter is the act the addressed control is for, and a bay \
                     is not one",
                )
            }
        }
        (Addressed::Head, Press::Enter) => {
            if !built.head.is_empty() {
                address_into(view, bay.name, 1);
                Asked::Moved
            } else {
                Asked::Nothing("this head draws no controls")
            }
        }
    }
}
