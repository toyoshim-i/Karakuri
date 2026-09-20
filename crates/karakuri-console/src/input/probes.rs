use karakuri_layout::Point;
use karakuri_operation::gate::Class;

use crate::panel::Panel;
use crate::view::{Field, KindChip, Scope, View, BAY_GRIPS, DECKS};

use super::handlers::*;

/// What each of rule 4's derivations answers for, one row per probe and in the
/// order [`claim`] asks them: the Outputs sink, the audio-in pill, the tracker
/// group's three, the arrangement pill, the look group's two, the transport
/// row's `rec` pill, a strip's five, the transition row's four, the Master
/// bay's one, the Inspector pane heads' `keep`, a deck head's seven, the
/// renderer chips, a parameter row's fader, the Program bay head's `solo`, the
/// four deck preview cells, the Library bay's scope chips, its two filter
/// fields, the `params` chip in its foot, the `load` button and deck pulldown
/// beside it, the list above them, the four class pills, the Sequencer bay's
/// cells, labels, mode pill, bank pills and `+ lane`, and the Staging lane's
/// `back` capsules and its rows.
///
/// One probe per derivation, cheapest answer first, and [`on_mcp`] is last
/// because it is the dearest probe here — each class lays out its bay's whole
/// head to find one capsule in it, and the Outputs one lays out the row's word
/// and its sink's name as well.
///
/// That is a cost ordering and not a correctness one. [`claim`] answers a
/// `bool` and the walk short-circuits, so no order over these rows can change
/// what it says: a press is on one of these controls or it is not, and which
/// probe noticed first is nobody's business outside this array. So the rows may
/// be reordered freely, and the only thing the order buys is that the common
/// press — which is on nothing — pays every cheap answer before it pays the
/// dear one. Which control a press then acts on is asked again by the caller,
/// in the caller's own order (`karakuri/src/main.rs`).
///
/// It is a table and not a sentence because it is what [`claim`] walks. A
/// derivation that is not a row here is never asked, so it is not a control at
/// all; and [`CONTROLS`] is summed over the same rows, so the number this crate
/// exports and the questions it is a count of cannot come apart. It was two
/// parallel arrays until 2026-09-07 — a hand-summed `[usize; N]` beside a
/// `[&dyn Fn; N]` inside [`claim`] — and one value carrying both is
/// [ADR-0274](../../../docs/adr/0274-a-control-is-a-row-in-the-consoles-own-table.md).
///
/// A control added inside a derivation already here is still a number to raise:
/// a sixth chip on a strip is one more thing a press reaches, and [`on_strip`]
/// would go on asking five questions while its row went on saying five. What
/// has changed is that the probe and the count are now one line apart instead
/// of two arrays apart, and that three of these rows no longer carry a number
/// at all — the class pills, the preview cells and the scope chips each count
/// the list their probe walks, so a fifth of any of the three raises
/// [`CONTROLS`] on its own. The rest are caught where every other fact about a
/// control is: the clearance test the rule above says each one owes,
/// `tests/mask.rs` being the most recent of them, and this row is what has to
/// be raised beside it.
///
/// The tracker group's row is the first one added under this table, and it is
/// what the table is for: the count moved from 36 to 39 and
/// `karakuri/src/main.rs`'s own array stopped compiling until the window said
/// how it asks the three. Nothing was scanned and nothing was summed by hand.
///
/// That is not a hypothetical, and the strip's row is the case it happened to.
/// [`crate::view::Mixer::select`] landed with the deck selection,
/// `karakuri/src/main.rs`'s press arm asked it, both manual pages described it,
/// and [`on_strip`] went on asking four questions — so no press on a strip's
/// ground ever reached [`Claim::Panel`] and the arm that would have acted on it
/// never ran. Nothing here could have caught it: the array's length was right
/// the whole time. What catches it now is `tests/mixer.rs`, which asks
/// [`claim`] at the points a strip has no chip on.
///
/// The scope row's count is [`Scope::ALL`]'s length and not a typed four. The
/// row is as long as the slice the host handed the bay, and what a host can
/// hand it is values of [`Scope`] — so the number of chips a pointer can reach
/// is the number of scopes that exist, and the day a fifth is added this rises
/// with it rather than being a four somebody has to remember. The class pills'
/// count is [`Class::ALL`]'s for the same reason, one crate out, and the
/// preview cells' is [`DECKS`].
///
/// The `params` chip's count is one, and it is one for a reason worth
/// separating from those: the chip is a *toggle over the cursor* rather than
/// one of a row, so however long the listing is there is one capsule to press.
/// What a press on it means depends on whether a reading is open, and that is
/// inside `LibraryBay::read` where the block is.
///
/// The `load` button and the deck pulldown are two on one row, and it is the
/// newest row here (ADR-0305). They were one *readout* until 2026-09-08 — `load
/// → A`, which said where the key would land and answered no pointer at all —
/// and the split is what made them controls. One row because they are one
/// derivation, and `claims: 2` because a pointer reaches two capsules through
/// it: the tracker group's three is the same shape, and so is a strip's five.
///
/// The pulldown counts one and not [`DECKS`], which is the `params` chip's
/// argument rather than the preview cells': the capsule is one press whatever
/// the mixer is drawing, and how many decks its list then offers is
/// `Load::picked`'s answer inside the card. The card itself is not counted at
/// all — rule 2 above claims every press while it is down, exactly as it does
/// for the two cards in the transport row. And the `→` between the two capsules
/// is not counted because it is not a control: it is a label, and
/// `tests/library.rs` sweeps it with the foot's ground.
///
/// The filter row's count is two and is a constant, unlike the scope row above
/// it: the row is `holds` and `layer` because
/// [`karakuri_operation::Operation::ListSets`] carries two things to narrow by,
/// and a third would be a change to the vocabulary rather than a longer slice
/// the host handed in. [`Field::ALL`] is the same two, and it is what the probe
/// walks.
///
/// The stars' count is one for the list's reason below, and it is the same
/// number for the same argument: a press lands on one of however many rows the
/// bay drew, and how many that is moves when a divider moves. The star is a
/// control of its own rather than a second reading of the row — it emits
/// `Operation::SetFavourite` where a row press emits nothing at all — so it is
/// a row of this table and not a sentence in the one under it.
///
/// The list's count is two, and neither of them is how many rows there are. One
/// rectangle, two controls: a press takes the Set in hand and a *secondary*
/// press puts that row's menu down (ADR-0311). They are two because a pointer
/// genuinely reaches two things there — which is the load button and the
/// pulldown's argument on one rectangle instead of two — and not two because
/// the row means two things under two scopes, which is [`on_row`]'s other pair
/// and is still one control. The card the menu puts down is not counted at all,
/// exactly as the other three cards are not: rule 2 claims every press while it
/// is there.
///
/// How many rows there are is the one number here that could have been a count
/// and must not be. A press lands on one of however many rows the bay drew,
/// exactly as it lands on one of however many chips it drew — and the chips are
/// counted while these are not, because the difference is what each number is
/// *about*. [`Scope::ALL`] is a closed list this crate owns, so the chip count
/// is a fact about the console; how many rows are drawn is `LibraryBay::rows`,
/// which is how many fit in a bay an operator can resize against a listing a
/// store answered — a number that changes while nobody presses anything.
/// [`CONTROLS`] is printed to an operator as *what the pointer reaches here*,
/// and a figure that moved when a divider moved would be answering a different
/// question. So the list is the control and which row is inside
/// [`crate::view::LibraryBay::take`], which is the `params` chip's row read the
/// other way round.
pub const PROBES: [Probe; 39] = [
    Probe {
        name: "the Outputs row's sinks",
        claims: 1,
        ask: on_sink,
    },
    Probe {
        name: "the audio-in pill",
        claims: 1,
        ask: on_audio,
    },
    Probe {
        name: "the tracker group's three",
        claims: 3,
        ask: on_tracker,
    },
    Probe {
        name: "the transport row's learn pill",
        claims: 1,
        ask: on_learn,
    },
    // **A readout and still a row here**, which is what this table is for: it
    // registers what the *pointer* reaches, and a press on the `map` pill
    // lands on the panel and does nothing. Handing it to `egui` instead would
    // make a press on a control the panel drew fall through to whatever is
    // behind it, and it is what carries the pill's tooltip.
    Probe {
        name: "the transport row's map pill",
        claims: 1,
        ask: on_map,
    },
    Probe {
        name: "the arrangement pill",
        claims: 1,
        ask: on_pill,
    },
    Probe {
        name: "the look group's two",
        claims: 2,
        ask: on_look,
    },
    Probe {
        name: "the transport row's rec pill",
        claims: 1,
        ask: on_rec,
    },
    Probe {
        name: "the transport row's tempo figure",
        claims: 1,
        ask: on_tempo,
    },
    Probe {
        name: "a mixer strip's five",
        claims: 5,
        ask: on_strip,
    },
    Probe {
        name: "the transition row's four",
        claims: 4,
        ask: on_transition,
    },
    Probe {
        name: "the Master bay's five",
        claims: 5,
        ask: on_master,
    },
    Probe {
        name: "the Inspector pane heads' name",
        claims: 1,
        ask: on_deck_name,
    },
    Probe {
        name: "the Inspector pane heads' keep",
        claims: 1,
        ask: on_keep,
    },
    Probe {
        name: "the Inspector pane heads' slot mcp policy",
        claims: 1,
        ask: on_slot_mcp,
    },
    // **The mark between the run and the count, and the card it puts down.**
    // Two controls and one row, which is the Library bay's `load` button and
    // deck pulldown's arrangement: one derivation, one ask.
    //
    // **Its place in this table is a cost ordering and nothing else** — the
    // header says so of every row — and the mark cannot be confused with the
    // run beside it either way: `deck_name` clips the run one gap short of
    // this rectangle, so the two are disjoint by construction rather than by
    // which probe answers first. While the card is down `claim`'s rule 2 has
    // already taken the press.
    Probe {
        name: "the Inspector pane heads' deck pulldown",
        claims: 2,
        ask: on_pane_target,
    },
    Probe {
        name: "a deck head's seven",
        claims: 7,
        ask: on_deck_head,
    },
    Probe {
        name: "the renderer chips",
        claims: 1,
        ask: on_rend,
    },
    Probe {
        name: "a parameter row's fader",
        claims: 1,
        ask: on_param,
    },
    Probe {
        name: "a parameter row's publish mark",
        claims: 1,
        ask: on_publish,
    },
    Probe {
        name: "a node group's `uses` capsule and its card",
        claims: 2,
        ask: on_uses,
    },
    Probe {
        name: "a node head's three authority chips",
        claims: crate::view::AUTHORITIES.len(),
        ask: on_auth,
    },
    // **The capsule at the right of the same head**, and the count is **one**
    // for the renderer row's reason and not the authority chips': how many
    // heads draw one is a property of the Set in the slot, where the three
    // levels are a closed list this console owns.
    Probe {
        name: "a node head's keep capsule",
        claims: 1,
        ask: on_node_keep,
    },
    Probe {
        name: "a sensitivity row's curve and take back",
        claims: 2,
        ask: on_sens,
    },
    Probe {
        name: "the Program bay head's solo",
        claims: 1,
        ask: on_solo,
    },
    Probe {
        name: "the grip in a bay head",
        claims: BAY_GRIPS,
        ask: on_grip,
    },
    Probe {
        name: "the deck preview cells",
        claims: DECKS,
        ask: on_cells,
    },
    Probe {
        name: "the Library bay's scope chips",
        claims: Scope::ALL.len(),
        ask: on_scope,
    },
    Probe {
        name: "the Library bay's filter fields",
        claims: Field::ALL.len(),
        ask: on_filter,
    },
    Probe {
        name: "the Library bay's kind chips",
        claims: KindChip::ALL.len(),
        ask: on_kinds,
    },
    Probe {
        name: "the Library bay's row badges",
        claims: 1,
        ask: on_badges,
    },
    Probe {
        name: "the params chip in the Library bay's foot",
        claims: 1,
        ask: on_read,
    },
    Probe {
        name: "the Library bay's load button and deck pulldown",
        claims: 2,
        ask: on_load,
    },
    Probe {
        name: "the Library bay's stars",
        claims: 1,
        ask: on_star,
    },
    Probe {
        name: "the Library bay's list",
        claims: 2,
        ask: on_row,
    },
    Probe {
        name: "the class pills",
        claims: Class::ALL.len(),
        ask: on_mcp,
    },
    Probe {
        name: "the Sequencer bay's cells, labels, minus glyphs, mode pill, bank pills and + lane",
        claims: SEQ_CONTROLS,
        ask: on_step,
    },
    Probe {
        name: "the Staging lane's back capsules",
        claims: 1,
        ask: on_back,
    },
    Probe {
        name: "the Staging lane's rows",
        claims: 1,
        ask: on_candidate,
    },
];

/// One of rule 4's derivations, as a value.
///
/// A control's registration is this row and nothing else: naming it, saying how
/// many controls a pointer reaches through it, and carrying the probe [`claim`]
/// asks. There is nowhere else to add one and nowhere else to forget one.
pub struct Probe {
    /// What the derivation answers for, in the words the rule above uses for it.
    ///
    /// It is what `karakuri/src/main.rs` keys its own half of the seam on. That
    /// file has to *act* on every control this file claims, and until 2026-09-07 it
    /// rebuilt the list by scanning this crate's source for `pub fn`s taking a
    /// `Point`, because there was no list here to read. A row is a value and has a
    /// name, so the scan is gone.
    pub name: &'static str,
    /// How many controls a pointer reaches through this one derivation, and what
    /// [`CONTROLS`] is a sum of.
    pub claims: usize,
    /// The derivation that draws those controls, asked whether the point is on one
    /// of them — and nothing is stored.
    ///
    /// A `fn` and not a closure, because every one of these rows wants exactly the
    /// four values [`claim`] itself takes: the panel for its solved layout, the
    /// `egui` context for a galley, the view for what the deck and the store said
    /// this frame, and the point. The derivations have nothing else in common —
    /// they answer nine different types to the caller — but the question *is the
    /// point on one of these* is one signature.
    pub ask: fn(&Panel, &egui::Context, &View, Point) -> bool,
}

/// How many controls the Sequencer bay claims: a cell per drawn step of every
/// lane, a label and a minus per lane, the mode pill, the four bank pills in
/// the bay head and the foot's `+ lane`.
///
/// The chooser's card is not counted, exactly as the Library's two are not: a
/// card that is down claims every press on the console under rule 2, which is
/// answered before this table is walked.
///
/// A count of what a *full* pattern draws rather than of what is on screen,
/// which is [`BAY_GRIPS`]' shape asked of a bay whose rows are data: this is a
/// `const` and a pattern arrives at run time, so the number registered is the
/// most a pointer could reach — four lanes, sixteen cells and two glyphs apiece
/// — and a console drawing one lane claims one lane's worth. That is the honest
/// direction for a registration: [`CONTROLS`] is what the pointer *may* have to
/// hit-test, and a number that followed the pattern would make the console's
/// own legend move when an operator added a lane.
///
/// `karakuri_console::view::Sequencer::controls` is what a drawn bay answers,
/// and it is the number this bounds.
const SEQ_CONTROLS: usize = DECKS * (karakuri_pattern::SLOTS + 2) + 1 + karakuri_pattern::BANKS + 1;

/// How many controls rule 4 hit-tests, summed over [`PROBES`].
///
/// Exported because the answer to *what can the pointer press here* is this
/// crate's and nobody else's: `egui` owns no widget anywhere on the console, so
/// a caller has no other way to ask. `karakuri/src/main.rs` prints it in its
/// legend, where the sentence it replaced said the panel had three controls and
/// went on saying it while ten more landed.
pub const CONTROLS: usize = summed(&PROBES);

/// [`PROBES`]' claims added up in a `const`, which `Iterator::sum` is not.
const fn summed(probes: &[Probe]) -> usize {
    let mut total = 0;
    let mut at = 0;
    while at < probes.len() {
        total += probes[at].claims;
        at += 1;
    }
    total
}
