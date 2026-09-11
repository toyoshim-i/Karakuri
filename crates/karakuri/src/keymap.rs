//! **The window loop's own keyboard**: [`KEY_BINDINGS`], the ten literal
//! keys `window_event` dispatches through it (`tab`, `esc`, `g`, `z`, `r`,
//! `k`, `b`, `,`, `.`, `n`), the [`KeyCtx`] each one's action takes instead
//! of `&mut App`, and the free function behind each row. [`key_column`] is
//! the unit test that holds the table against
//! `docs/manual/operations.html` directly, and the manual's own key column
//! against the four keys `crate::grammar` answers for beside it.
//!
//! Split out of `main.rs` on 2026-09-11, continuing that file's own
//! `mod`-based decomposition — `karakuri-console/src/view.rs`'s recent
//! bay-by-bay split, one binary crate along. `crate::session`'s `Sessions`
//! and `Keeping` were the first piece out; this is the second.
//!
//! **Not `window_event`'s dispatch call site**, which stays in `main.rs`:
//! the lookup into [`KEY_BINDINGS`], the [`KeyCtx`] built from `self`'s
//! fields, and the `match` on [`KeyAction`] that calls through it all read
//! this table from the other side of the crate boundary, exactly as
//! `main.rs` already reads `crate::session::Sessions` and
//! `crate::session::Keeping`. Not `App`, not `Readout` and its translator
//! cluster, not `fn main` — separate, larger work still ahead of it.
//!
//! [`key_column`]: key_column

use std::time::Instant;

use karakuri_console::focus;
use karakuri_console::panel::Op;
use karakuri_console::repaint::{Change, Repaint};
use karakuri_environment::Asked;
use karakuri_operation::{GridScale, Operation};
use winit::keyboard::{Key, NamedKey};

use crate::session::{Keeping, Sessions};
use crate::{Acted, App, Costs, Gfx, Readout};

/// **A key `window_event` binds outside the grammar guard** — the ~ten
/// literal `Key::Character("…")` and `Key::Named(NamedKey::…)` arms the
/// `match` on `key.logical_key` used to spell inline, one each in
/// [`KEY_BINDINGS`] now.
///
/// **Why a table and not the arms it replaced**: those differed only in
/// which key they answered to and what they did about it, and a bare
/// `match` said that in five hundred lines a test could read only by
/// re-parsing this file as text — `key_column::bound`'s old shape, which
/// stopped at the first `#[cfg(test)]` and broke on a stray comment or a
/// reordered arm. The facts are the same; they are data now, and
/// `key_column::bound` reads them as data instead of as this file's own
/// source.
///
/// **Not the grammar guard**, which stays exactly the `match` arm it always
/// was — `named if grammar(&named).is_some()` — because it is already
/// table-driven one layer down, through `karakuri_console::focus`, and
/// binds a different key in every bay it reaches. Nothing here duplicates
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoundKey {
    Named(NamedKey),
    Character(&'static str),
}

impl BoundKey {
    /// Whether `key`, as `window_event` hands it in from
    /// `key.logical_key.as_ref()`, is this one.
    pub(crate) fn matches(self, key: &Key<&str>) -> bool {
        match (self, key) {
            (BoundKey::Named(named), Key::Named(other)) => named == *other,
            (BoundKey::Character(text), Key::Character(other)) => text == *other,
            _ => false,
        }
    }
}

/// **Exactly the fields a bound key's action needs, and not `self`.**
///
/// `window_event` is already holding a live `&mut Gfx` reborrowed out of
/// `self.gfx` by the time a key is dispatched (see its own opening lines),
/// so an action taking `&mut App` would have to borrow all of `self` a
/// second time and collide with that borrow. `App::performed` and
/// `App::wants` solved the same problem the same way, by naming the
/// individual fields they touch instead of taking `&mut self` — this is
/// that solution collected into one struct because ten call sites named the
/// same eight fields.
pub(crate) struct KeyCtx<'a> {
    pub(crate) readout: &'a mut Readout,
    pub(crate) egui_due: &'a mut Option<Instant>,
    pub(crate) costs: &'a mut Costs,
    pub(crate) recording: &'a mut Sessions,
    pub(crate) keeping: &'a mut Keeping,
    pub(crate) store: &'a std::path::Path,
    pub(crate) started: Instant,
    pub(crate) shift: bool,
}

/// **What pressing a bound key does**, once [`KEY_BINDINGS`] has found its
/// entry.
///
/// The arms this table replaced were not one shape: `Op::FoldEnclosing`,
/// `Op::UnfoldAll` and `Op::Reset` fell through to `self.readout.op(op)`
/// below the old `match`, and the rest — a view moved, an `Operation`
/// emitted, a frame asked for — handled the whole press themselves and
/// returned. Rather than force one payload on bodies that were never one
/// shape, each variant here carries the function pointer for the shape it
/// is.
#[derive(Clone, Copy)]
pub(crate) enum KeyAction {
    /// Produces the `panel::Op` that falls through to `self.readout.op(op)`,
    /// or `None` where the press has nothing to act on (`g` off every bay).
    Panel(fn(&mut KeyCtx) -> Option<Op>),
    /// Handles the whole press — asks for whatever frame it owes, if any —
    /// and the window loop returns immediately after calling it.
    Handled(fn(&mut KeyCtx, &mut Gfx)),
    /// Moves focus between bays or within a bay and returns whether focus moved.
    Focus(fn(&mut KeyCtx) -> bool),
}

/// **One row of the window loop's own keyboard**, checked against
/// `docs/manual/operations.html` directly by `key_column`'s tests rather
/// than through a second list nothing holds against the page.
pub(crate) struct KeyBinding {
    pub(crate) key: BoundKey,
    /// The word this key is bound under, both in `key_column::KEYS`'s
    /// legend and in the page's own key-column badges —
    /// `docs/manual/operations.html`'s spelling, not `winit`'s.
    ///
    /// **Read only by `key_column`'s tests** — dispatch itself never asks
    /// this table what a key is *called*, only which one `matches`.
    #[cfg_attr(not(test), allow(dead_code))]
    legend: &'static str,
    /// The bay a badge naming this key is addressed to — `Some(focus::ANY)`
    /// for the one key of this table that is not global, `None` for every
    /// other one. Read only by `key_column`'s tests, for `legend`'s reason.
    #[cfg_attr(not(test), allow(dead_code))]
    bay: Option<&'static str>,
    /// The `<h3>` title this key reaches on the page, or `None` for a
    /// view-only action the page does not specify as an operation — moving
    /// focus, abandoning a name, the room's colours. Read only by
    /// `key_column`'s tests, for `legend`'s reason.
    #[cfg_attr(not(test), allow(dead_code))]
    title: Option<&'static str>,
    pub(crate) action: KeyAction,
}

/// **Every key `window_event` binds outside the grammar guard.**
///
/// Ten entries for the ten literal arms `window_event`'s `match` used to
/// spell — `tab`, `esc`, `g`, `z`, `r`, `k`, `b`, `,`, `.`, `n` — the same
/// ten `key_column::KEYS` prints beside the eight the grammar guard answers
/// for. What each one reaches on `docs/manual/operations.html` is
/// [`KeyBinding::title`], checked in `key_column` rather than assumed.
pub(crate) const KEY_BINDINGS: &[KeyBinding] = &[
    // **`Tab` moves focus to the next bay, and `shift-Tab` to the one
    // before** — ADR-0259's first key, and the whole of what makes the six
    // that follow it addressable.
    //
    // **The walk is the console's and the key is this file's**, which is the
    // seam every control on this panel already crosses (ADR-0156): the ring
    // is derived from the arrangement by `karakuri_console::focus::ring`,
    // and what `key_tab` knows is which direction was asked for.
    //
    // **It names no operation and asks for no record**, which is the two
    // library cursor keys' arrangement one bay out: focus is a pointer this
    // console owns, nothing downstream can be the model of record for it,
    // and `Change::Pointed` is the answer for *a key moved a pointer*. The
    // page says the same thing by leaving this row's key column alone —
    // there is no *move focus* row, because moving focus is not an
    // operation. Hence `title: None`.
    //
    // **`egui` never gets a say.** `egui-winit` 0.36.1 reports `consumed`
    // for every `Tab` whatever has focus, and `App::to_egui` destructures
    // `EventResponse` down to `repaint` and nothing else — see that
    // function's own doc for why that shape is what makes this key
    // reachable at all.
    KeyBinding {
        key: BoundKey::Named(NamedKey::Tab),
        legend: "tab",
        bay: None,
        title: None,
        action: KeyAction::Focus(key_tab),
    },
    // **`esc` goes up one level of the focused bay's address, and it does
    // not quit** (ADR-0259). Quitting follows the platform's own
    // accelerator — `⌘Q`, `Alt-F4` — which arrives as
    // `WindowEvent::CloseRequested` and is answered at the top of
    // `window_event`, saves waited for and recording flushed. **A quit
    // ladder is a sequence that ends in something irreversible, in front of
    // an audience, reached by repeating one key**
    // ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    //
    // **At bay level it acts on nothing and says so**, because there is no
    // unfocused state to fall out into and a key that declines silently is
    // indistinguishable from one that is not bound.
    //
    // **While a name is being typed it abandons the name**, and that is not
    // an exception — both letter-taking flows return before this `match` is
    // ever reached, and ADR-0259 reads them as a *field*: *"`esc` … from a
    // field it abandons the name"*.
    KeyBinding {
        key: BoundKey::Named(NamedKey::Escape),
        legend: "esc",
        bay: None,
        title: None,
        action: KeyAction::Focus(key_escape),
    },
    // **The one letter left that names a region, and it takes it from the
    // focus rather than from the pointer** (ADR-0259, ADR-0343). `g` folds
    // the split enclosing the focused bay, which is a pane every time — a
    // bay's parent is a split and never another bay — so this is the one
    // route to *Fold a pane away* from the keyboard alone, and it is the row
    // `space` on a bay does not reach. **Rule 01 is what it buys** — *every
    // operation is reachable from the keyboard alone* — because a key whose
    // region came from the pointer was not.
    KeyBinding {
        key: BoundKey::Character("g"),
        legend: "g",
        bay: Some(focus::ANY),
        title: Some("Fold a pane away"),
        action: KeyAction::Panel(key_fold_enclosing),
    },
    KeyBinding {
        key: BoundKey::Character("z"),
        legend: "z",
        bay: None,
        title: Some("Bring back what is folded"),
        action: KeyAction::Panel(key_unfold_all),
    },
    KeyBinding {
        key: BoundKey::Character("r"),
        legend: "r",
        bay: None,
        title: Some("Reset the arrangement"),
        action: KeyAction::Panel(key_reset),
    },
    // **Keep what the selected deck is playing**, filed under a stamp
    // because a bare key press cannot type a name — see
    // `karakuri_environment::accepted_save`, whose convention that is and
    // whose reason it borrows: an operator looks for the time they saved it.
    //
    // **The selected deck and not a slot in the key**, which is the split
    // every deck-addressed control on this panel makes: the deck an
    // operator means is the one they have already addressed in the Mixer,
    // and a model has no selection and names the slot in the call.
    //
    // **The panel column of this row is still `plan`.** A key is not a
    // control, the Library bay has no *keep* pill drawn, and a badge that
    // said otherwise would be a claim about a control that is not there.
    KeyBinding {
        key: BoundKey::Character("k"),
        legend: "k",
        bay: None,
        title: Some("Keep what a deck is playing"),
        action: KeyAction::Handled(key_save),
    },
    // **The beat, tapped.** The one key on this panel that reaches the room
    // rather than the deck or the arrangement, and the first of three that
    // need an input open. What it does and why it does not go through
    // `written` is [`tapped`].
    KeyBinding {
        key: BoundKey::Character("b"),
        legend: "b",
        bay: None,
        title: Some("Tap the beat"),
        action: KeyAction::Handled(key_tap_beat),
    },
    // **The grid, an octave either way**, and the two keys the page
    // specifies for it. Refused where the result would leave the trackable
    // range, which is the beat lock's call — see [`scaled`].
    KeyBinding {
        key: BoundKey::Character(","),
        legend: ",",
        bay: None,
        title: Some("Halve or double the grid"),
        action: KeyAction::Handled(key_scale_grid_halve),
    },
    KeyBinding {
        key: BoundKey::Character("."),
        legend: ".",
        bay: None,
        title: Some("Halve or double the grid"),
        action: KeyAction::Handled(key_scale_grid_double),
    },
    // **The key that changes the screen without touching the pointer and
    // without touching the model.** The room is the view's: every colour on
    // the panel changes and nothing in the arrangement moves, so no
    // `Outcome` says so and `Change::Room` is the only thing that does —
    // hence `title: None`, the same as `Tab` and `Escape`.
    KeyBinding {
        key: BoundKey::Character("n"),
        legend: "n",
        bay: None,
        title: None,
        action: KeyAction::Handled(key_room),
    },
];

// -- KEY_BINDINGS' actions, one free function per entry ---------------------
//
// Free functions and not methods, and each takes a [`KeyCtx`] rather than
// `&mut App`, for the reason [`KeyCtx`] itself gives: `window_event` is
// already holding a `&mut Gfx` reborrowed out of `self.gfx` by the time one
// of these is called. Each body is exactly what the arm it replaced had —
// see [`KEY_BINDINGS`] for the doc comment that used to sit on the arm
// itself.

pub(crate) fn key_tab(ctx: &mut KeyCtx) -> bool {
    let step = match ctx.shift {
        true => -1,
        false => 1,
    };
    ctx.readout.view.tab(&ctx.readout.panel, step)
}

pub(crate) fn key_escape(ctx: &mut KeyCtx) -> bool {
    let moved = ctx.readout.view.focus_up(&ctx.readout.panel);
    if !moved {
        println!(
            "  esc: the address is already at the {} bay and there is no \
             level above it — press tab to move focus to another bay, or \
             close the window to quit",
            ctx.readout
                .view
                .focused(&ctx.readout.panel)
                .map_or("focused", |bay| bay.name)
        );
    }
    moved
}

fn key_fold_enclosing(ctx: &mut KeyCtx) -> Option<Op> {
    ctx.readout
        .view
        .focused(&ctx.readout.panel)
        .and_then(|bay| ctx.readout.panel.layout().find(bay.name))
        .map(Op::FoldEnclosing)
}

fn key_unfold_all(_ctx: &mut KeyCtx) -> Option<Op> {
    Some(Op::UnfoldAll)
}

fn key_reset(_ctx: &mut KeyCtx) -> Option<Op> {
    Some(Op::Reset)
}

fn key_save(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    let deck = ctx.readout.view.selection();
    // **`None`, and it is the payload saying so rather than this function
    // inventing a stamp.** A caller that can type a name is not made to
    // take a timestamp, and a key press is not one of them.
    let acted = Acted::Emitted(Some(Operation::SaveSet { deck, id: None }));
    // **Named through `performed` and performed beside it**, which is
    // `e`'s shape: the emission is what records the press as
    // `Silent(OnLanding)` rather than as nothing at all, and the save
    // itself is this function's because `Operation::SaveSet` writes no
    // record here — the `save` record is written where the work lands,
    // and this program records no session to write it into.
    let repaint = App::performed(
        gfx,
        ctx.started,
        ctx.readout,
        ctx.recording.recorder(),
        &acted,
        Repaint::Never,
    );
    ctx.keeping.save_set(
        &gfx.engine,
        ctx.store,
        Asked::Operator,
        usize::from(deck),
        None,
        None,
    );
    App::wants(gfx, ctx.egui_due, ctx.costs, repaint);
}

fn key_tap_beat(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    let acted = Acted::Emitted(Some(Operation::TapBeat));
    let repaint = App::performed(
        gfx,
        ctx.started,
        ctx.readout,
        ctx.recording.recorder(),
        &acted,
        Repaint::Never,
    );
    App::wants(gfx, ctx.egui_due, ctx.costs, repaint);
}

/// The shared half of [`key_scale_grid_halve`] and [`key_scale_grid_double`]
/// — the two keys' one difference is `by`.
fn key_scale_grid(ctx: &mut KeyCtx, gfx: &mut Gfx, by: GridScale) {
    let acted = Acted::Emitted(Some(Operation::ScaleGrid { by }));
    let repaint = App::performed(
        gfx,
        ctx.started,
        ctx.readout,
        ctx.recording.recorder(),
        &acted,
        Repaint::Never,
    );
    App::wants(gfx, ctx.egui_due, ctx.costs, repaint);
}

fn key_scale_grid_halve(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    key_scale_grid(ctx, gfx, GridScale::Halve);
}

fn key_scale_grid_double(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    key_scale_grid(ctx, gfx, GridScale::Double);
}

fn key_room(ctx: &mut KeyCtx, gfx: &mut Gfx) {
    ctx.readout.room();
    App::wants(gfx, ctx.egui_due, ctx.costs, Change::Room.repaint());
}

#[cfg(test)]
pub(crate) mod key_column {
    //! **The key column of the manual, against the keys this window binds.**
    //!
    //! [ADR-0213](../../../docs/adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
    //! defined the *panel* column of `docs/manual/operations.html` — `has`
    //! means an operator running the instrument reaches the operation — and
    //! said nothing about the other three. The *key* column then stopped being
    //! well defined, because
    //! [ADR-0214](../../../docs/adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md)
    //! gave this workspace a second keyboard: `karakuri-cli` binds thirty-nine
    //! keys and this program binds every key in [`crate::KEYS`], **seven
    //! letters mean different things on the two**, and a badge saying
    //! `key f g` did not say whose.
    //! (Nine when ADR-0220 was written; the library's load route added seven —
    //! the four that select a deck, the two that walk the library cursor, and
    //! `l`. The four are also the one place where the two keyboards
    //! **agree**, because a deck is a slot number and there was nothing to
    //! translate — `esc` was the other until 2026-09-09. Eight when the audio session landed and `b`,
    //! `,` and `.` joined them, and seven since `p` stopped being the panel's
    //! report and became the latency offset the page specifies — the one
    //! letter this column has ever taken *back* from the panel, and the pair
    //! `o` and `p` agree on both keyboards now. **Eight since 2026-09-09**,
    //! when `esc` stopped quitting here and became ADR-0259's *up one level*:
    //! it is the one key the two keyboards agreed on that they no longer do,
    //! and the command line's is still its own — `karakuri-cli` is test tooling
    //! and the instrument's principles do not bind it (ADR-0242).)
    //!
    //! The page now says whose, in its legend: **the key column is the
    //! instrument's keyboard**, which is `window_event`'s `match` on
    //! `key.logical_key` — the ten literal keys through
    //! [`super::KEY_BINDINGS`] since 2026-09-10, the grammar's own four
    //! through the guard beside it. That is ADR-0213's definition one column along — the
    //! property is *the operator at the panel presses it*, and *which cargo
    //! target binds a letter* is the shape ([P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)).
    //! This is the check that definition owes, both ways round, in the shape
    //! `karakuri-environment/src/mcp.rs` uses for the MCP column and
    //! `karakuri-console/tests/panel_column.rs` for the panel one.
    //!
    //! # Why the check is here and can be nowhere else
    //!
    //! The keys are in this file, and **nothing in this workspace may depend on
    //! this package** — it is a binary with no library target on purpose, as
    //! the crate header says: *a surface is where the buck stops*. The two
    //! files that check the panel column both stop at exactly this boundary and
    //! say so: `panel_column.rs` — *"reachability is a property of
    //! `crates/karakuri/src/main.rs` … and this crate takes no device and
    //! cannot depend on that binary (ADR-0156). So this file checks the
    //! necessary half and not the sufficient one"* — and `vocabulary.rs` the
    //! same. A key column check has that problem twice over, because the other
    //! keyboard is in `karakuri-cli`, which no crate can depend on either.
    //!
    //! So it is a unit test in the binary that holds the keys. It cannot be an
    //! integration test under `crates/karakuri/tests/`, because a package with
    //! no library target has nothing for one to `use`; the arms are reachable
    //! only from inside this file's own `#[cfg(test)]`.
    //!
    //! **The command line's keyboard is not this file's and not this column's.**
    //! `karakuri-cli` documents its own keys in `BINDINGS` and has its own test
    //! that every key `Live::key` acts on is in it. Nothing here reads that
    //! package, and a second copy of its list here would be the thing
    //! [`docs/contributing.md` §4](../../../docs/contributing.md)
    //! forbids.
    //!
    //! # What it cannot see, and which way each one fails
    //!
    //! - **A key `window_event` dispatches that is declared in none of
    //!   [`bound`]'s four sources** — [`super::KEY_BINDINGS`],
    //!   [`GRAMMAR_KEYS`], [`DIGIT`] and [`NAME_ENTRY_KEY`] — through `egui`'s
    //!   own shortcut handling, say. Invisible to [`bound`], and a *false
    //!   negative*: it cannot fail the direction that says every bound key is
    //!   on the page, and it surfaces from the other direction the moment
    //!   somebody marks that row built. **This one key wider than it was**:
    //!   the ten keys of [`super::KEY_BINDINGS`] cannot drift from what
    //!   `window_event` dispatches — the same array is both, so there is
    //!   nothing left to scan for and nothing left to miss — but
    //!   [`GRAMMAR_KEYS`] and [`NAME_ENTRY_KEY`] are declared facts about
    //!   `crate::grammar` and the two letter-taking flows rather than
    //!   anything read out of them, so a key those stop binding, or start
    //!   binding a different one, is invisible here exactly as it always was
    //!   for [`DIGIT`].
    //! - **This file does not press a key.** It reads the table, reads the
    //!   grammar guard, and reads the page. That `Op::Solo` actually solos is
    //!   `karakuri-console/tests/vocabulary.rs`'s, which asks a running `Panel`;
    //!   that a binding is reached at all is what
    //!   `tests::a_drag_through_the_window_loops_own_routing_never_reaches_egui`
    //!   asks about the pointer, and nothing asks it for keys.
    //!   **`egui` sees every key before `window_event`'s `match` does**, and if it ever
    //!   grew a focused widget that consumed one, the binding would still be
    //!   here and this file would go on claiming an operator reaches it. That
    //!   is the sufficient half, and it is not checked here either — one
    //!   boundary further out than the two files above stop at.
    //! - **Which rows a key lands on is written down rather than derived**, in
    //!   [`ROWS`]. It has to be: `Op::Fold` folds a bay or a pane depending on
    //!   what the pointer is over, and only the page separates those two rows.
    //!   A wrong entry is a wrong claim, and it cannot be *quietly* wrong —
    //!   both assertions below read the same list, so an entry naming a row
    //!   that is not marked built fails one and a badge naming a key no entry
    //!   claims fails the other.
    //! # Two rows this program deliberately binds no key to
    //!
    //! *Save the arrangement* and *Put a saved arrangement back* each carry a
    //! name the operator picked, and **a bare key press cannot type one**.
    //! ADR-0221 §1 provides a fallback — a surface that cannot type a name
    //! passes a `history::stamped_id` stamp, as `accepted_save` does for a Set
    //! — and it is declined here for two reasons, both of which would show up
    //! as a badge that lies:
    //!
    //! - **Putting one back cannot be bound at all.** Nothing at a key press
    //!   says *which* arrangement, and *the most recent* is a handle derived
    //!   from where a file sits, which is the failure
    //!   [P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)
    //!   is about and the one ADR-0221 rejected a slot number over.
    //! - **So a save key alone would keep arrangements nothing can put back.**
    //!   This program has no control that lists them and no way to show an
    //!   operator the stamp it picked for them, and `ArrangementEntry`'s own
    //!   documentation says an arrangement is *"saved by an operator who is
    //!   telling the console what to call this shape"*. A `has` badge would be
    //!   true of the press and false of everything the press was for.
    //!
    //! Both rows therefore carry four empty badges on [`PAGE`], and the page
    //! says the same thing in its own words. Binding either one is a change to
    //! that specification first, and it wants the control this page's *panel*
    //! badges now name — the transport row — rather than a letter.
    //!
    //! - **Only the key column.** The panel column is
    //!   `karakuri-console`'s two files, the MCP column is
    //!   `karakuri-environment/src/mcp.rs`, and **the MIDI column is checked by
    //!   nothing** — which this file says rather than being read as covering
    //!   it.

    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use karakuri_console::focus::ANY;

    use crate::KEYS;

    /// The specification, relative to the workspace root.
    const PAGE: &str = "docs/manual/operations.html";

    /// What marks a row on the page — the marker `panel_column.rs`,
    /// `vocabulary.rs` and `mcp.rs` all match, for the reason the first of them
    /// gives: sections are `<h2>` and a heading somebody adds for looks is
    /// neither.
    const ROW: &str = r#"<div class="op-head">"#;

    /// The badge text of a route that names nothing. A `plan` or `gap` badge is
    /// allowed to be this; a `has` badge is not, because it would claim an
    /// operator reaches the operation and decline to say what to press.
    const NOWHERE: &str = "&mdash;";

    /// **The digit, declared rather than scanned.**
    ///
    /// [`bound`] answers *which keys does this program bind* out of
    /// [`super::KEY_BINDINGS`] and [`GRAMMAR_KEYS`] now, and the digit is
    /// the one key of the grammar neither carries: `crate::grammar` binds it
    /// with a guard rather than a literal, deliberately, because ten
    /// literals would say the digits are bound and say nothing about what
    /// they reach — a digit reaches a different row in every bay, and the
    /// dispatch table is what knows which (ADR-0259, ADR-0333). So this one
    /// entry is still conditioned on `karakuri_console::focus::BUILT` rather
    /// than asserted outright, in [`bound`] itself.
    const DIGIT: &str = "digit";

    /// **How the page spells each key of the grammar**, and what a badge
    /// naming one resolves to.
    ///
    /// The digits are all one key and both arrow pairs are the arrows, which
    /// is [`ROWS`]' own shape: a row is reached by *the arrows in the Mixer*,
    /// and which pair depends on whether the thing addressed is an item laid
    /// out in a row or a level standing on its own. **That is the one thing
    /// this table gives up**, and it is written down rather than left to be
    /// found: a badge naming `&larr;&rarr;` on a row the arrows reach only by
    /// stepping a level passes here. What the axis is, is
    /// `karakuri_console::focus::Built::across`, and nothing holds the page
    /// against it.
    const SPELLED: &[(&str, &str)] = &[
        ("&uarr;&darr;", "arrows"),
        ("&larr;&rarr;", "arrows"),
        ("up", "arrows"),
        ("down", "arrows"),
        ("left", "arrows"),
        ("right", "arrows"),
        ("space", "space"),
        ("enter", "enter"),
    ];

    /// **How a badge names the bay a press is addressed in**, and the whole of
    /// the grammar this column's designed half is written in: a key, a
    /// separator and a bay. `space &middot; in the Mixer`.
    const IN_THE: &str = " &middot; in ";

    /// **The bay a badge's bay-name resolves to**, in the arrangement's own
    /// names — which is what `karakuri_console::focus::BUILT` is keyed by.
    ///
    /// The page writes them the way a person says them, with the article the
    /// bay's own sentence uses: *in the Mixer*, *in Staging*. Two spellings for
    /// nine bays, and the article is the page's rather than something to
    /// normalise away.
    const BAYS: &[(&str, &str)] = &[
        ("the Transport", "transport"),
        ("the Library", "library"),
        ("Staging", "staging"),
        ("the Program", "program"),
        ("the Inspector", "inspector"),
        ("the Mixer", "mixer"),
        ("the Master", "master"),
        ("the Sequencer", "sequencer"),
        ("the Outputs", "outputs"),
    ];

    /// **How a badge names a press that is addressed in every bay**, and what
    /// it resolves to.
    ///
    /// `space` at bay level is the fold and `g` is the split enclosing the
    /// focused bay: neither is one bay's, and neither is global — the operand
    /// is the bay that has focus, which is the third category ADR-0343 names.
    /// So the page spells it `&middot; in any bay` and it resolves to
    /// `karakuri_console::focus::ANY`, which is **not** one of [`BAYS`]' nine
    /// and is deliberately not in that table: a badge naming one of the nine is
    /// a claim about one place, and this is a claim about all of them.
    ///
    /// **The reverse check reads it as all nine**, which is the stronger
    /// reading and the honest one — see
    /// [`every_key_route_the_page_marks_built_is_bound_by_the_instrument`].
    const ANY_BAY: (&str, &str) = ("any bay", karakuri_console::focus::ANY);

    /// **Every route this program binds, and the rows of [`PAGE`] it reaches.**
    ///
    /// `None` for a bay is a **global** key — one whose meaning does not depend
    /// on where the address is, which is the operand rule ADR-0259 closes the
    /// global list by. `Some(bay)` is a key of the grammar, addressed to that
    /// bay, and the key is spelled as the grammar spells it rather than as a
    /// letter.
    ///
    /// # Why the key alone no longer determines a row
    ///
    /// `space` in the Mixer is a residency, a blend mode, a mask shape or a
    /// level's default; `space` in the Library is a scope. One key, two bays,
    /// six rows — so a mapping keyed on the key alone would either name all six
    /// for both bays or name none. **That is the whole of why this table grew a
    /// column**, and it is ADR-0259's own consequence: *"`ROWS` becomes keyed
    /// by a (bay, key) pair with the globals under no bay."*
    ///
    /// # What is written down and what is derived
    ///
    /// The **rows** are written down and cannot be derived: `Op::Fold` folds a
    /// bay or a pane depending on what the pointer is over, and only the page
    /// separates those two. The **pairs** are derived —
    /// `karakuri_console::focus::reaches` flattens the dispatch table — and
    /// [`the_grammar_the_page_names_is_the_grammar_the_console_declares`] holds
    /// the two against each other in both directions, so a bay whose grammar is
    /// built and has no rows here fails, and a pair here the console does not
    /// declare fails.
    ///
    /// The rows are the page's headings byte for byte.
    const ROWS: &[(Option<&str>, &str, &[&str])] = &[
        // ------------------------------------------------------------------
        // The globals: a letter whose operation has no operand for focus to
        // supply, or whose only operand is the choice the key itself spells.
        // ------------------------------------------------------------------
        // The room's colours. Nothing in the arrangement moves and no
        // `Outcome` says so, which is why it is not an operation.
        (None, "n", &[]),
        (None, "r", &["Reset the arrangement"]),
        // `Op::UnfoldAll` — the page carries the region and the everything
        // under one heading, as `vocabulary.rs` does.
        (None, "z", &["Bring back what is folded"]),
        // **The three that need a room**, and they are the keys here that
        // reach neither the arrangement nor the deck. `b` is a tap and `,`
        // and `.` are the octave; each performs against the audio session
        // this program opened, and each says so when there is none rather
        // than doing nothing (`crate::tapped`, `crate::scaled`).
        (None, "b", &["Tap the beat"]),
        (None, ",", &["Halve or double the grid"]),
        (None, ".", &["Halve or double the grid"]),
        // **The save, whose operand is the deck selection.**
        //
        // **It is the key column and not the panel column that this makes
        // `has`.** The Library bay draws no *keep* control, so the row's panel
        // badge stays `plan` — a key is not a control, and a badge that named
        // one would be a claim about something that is not drawn. **Which is
        // also why it is still a letter**: ADR-0259 makes this *"an act on a
        // control the Library bay does not draw yet"*, and a grammar key
        // cannot be addressed to a control nobody draws.
        (None, "k", &["Keep what a deck is playing"]),
        // **The two that move the address**, and neither names a row: focus is
        // a pointer this console owns and moving one is not an operation
        // (ADR-0332).
        (None, "esc", &[]),
        (None, "tab", &[]),
        // **The three that are live only while a name is being typed.**
        // `enter` and `space` are grammar keys the rest of the time and appear
        // under their bays below; this is the rub-out, which is nothing else.
        //
        // ADR-0221 records that **no key is bound to saving or restoring an
        // arrangement**, and that is still true: it does not *name* the
        // operation and cannot be pressed to reach it. A save is reached by
        // opening the pill's menu and picking *save*, which is a pointer.
        (None, "backspace", &[]),
        // ------------------------------------------------------------------
        // Addressed to the focused bay, wherever it is
        // ------------------------------------------------------------------
        // **`space` on a bay is the fold, in every one of the nine**, which is
        // ADR-0259's rule and the narrow reason a folded bay keeps its place
        // in the ring. It is one route naming all nine rather than nine routes
        // naming one row: a badge reading `space &middot; in the Mixer` on
        // *Fold a bay away* would name one of nine places the press works.
        (Some(ANY), "space", &["Fold a bay away"]),
        // **And `g` is the split enclosing the focused bay**, which is a pane
        // every time — a bay's parent is a split and never another bay. It is
        // the one route to this row from the keyboard alone, and it is why the
        // letter survived `f` (ADR-0343).
        (Some(ANY), "g", &["Fold a pane away"]),
        // ------------------------------------------------------------------
        // The Transport's grammar
        // ------------------------------------------------------------------
        // A headless row, so `0` names the row itself and a digit names one of
        // the controls left to right. Naming one asks for nothing.
        (Some("transport"), DIGIT, &[]),
        // **`↑↓` on the exposure and on the offset**, which are this row's two
        // levels. The tempo figure is a track a press positions and not a
        // level with a step, so the arrows decline on it and say so.
        (
            Some("transport"),
            "arrows",
            &["Exposure", "Nudge the latency offset"],
        ),
        // **`space` on the tone map cycles the four operators**, and on the
        // exposure it is the value the control was declared at.
        (Some("transport"), "space", &["Tone map", "Exposure"]),
        // ------------------------------------------------------------------
        // The Library's grammar
        // ------------------------------------------------------------------
        // **A digit names the nth row and `0` the head**, and neither asks for
        // an operation: the cursor is a pointer nothing in the vocabulary
        // moves, which is `console.html`'s *How a Set reaches a deck*.
        (Some("library"), DIGIT, &[]),
        // The rows, walked — today's `up` and `down`, and the same nothing.
        (Some("library"), "arrows", &[]),
        // **`space` on the head's scope chips and on a row's star.** The chips
        // are drawn by `karakuri-console` and pressed by nobody: `SelectScope`
        // is emitted from `Readout::pointer` and never from a control, so that row's
        // panel column stays `plan` and this key is what makes its key column
        // `has`.
        (
            Some("library"),
            "space",
            &[
                "Choose which scope the library shows",
                "Star a Set, or take the star off",
            ],
        ),
        // **`enter` on a row is the load**, with both operands on screen
        // before the press — the deck selection says which deck and the
        // address says which Set. **`enter` on a row's `params` chip opens
        // what that Set holds and declares**, which is the second act a row
        // has and the reason a row's controls are numbered at all.
        //
        // **One key, and a preset row reaches a second page row through it.**
        // Taking a Set in is not a row of its own — ADR-0229's *one operation,
        // two moments* — so a press on a `presets` row performs *Send a Set to
        // somebody, and take one in* at the moment of the press and then the
        // load. That row's key badge names no key: what an operator reaches
        // from the keyboard is a **load**.
        (
            Some("library"),
            "enter",
            &[
                "Load material into a deck",
                "Read what one Set holds and declares",
            ],
        ),
        // ------------------------------------------------------------------
        // The Staging lane's grammar
        // ------------------------------------------------------------------
        // **Nothing here has a state at all**, so this bay has no `space`
        // route below the fold — ADR-0259's own finding, and the second of the
        // two bays that are lists of things that happened rather than things
        // you set.
        (Some("staging"), DIGIT, &[]),
        (Some("staging"), "arrows", &[]),
        // **A row's two acts are two controls and a digit chooses between
        // them**, which is the record's `n 1` and `n 2`.
        (
            Some("staging"),
            "enter",
            &["Keep a candidate", "Put a node's previous version back"],
        ),
        // ------------------------------------------------------------------
        // The Program bay's grammar
        // ------------------------------------------------------------------
        // **The four cells answer to nothing** and the picture's on and off is
        // the Outputs row's one control, so a digit lands, the ring is drawn,
        // and `space` and `enter` decline and say why. It is the clearest case
        // in the walk of items with neither a state nor an act.
        (Some("program"), DIGIT, &[]),
        (Some("program"), "arrows", &[]),
        // **`space` on the head's `solo`**, which is `s` and `u` collapsed
        // into the one control they always described (ADR-0259). The class
        // pill beside it opens a class rather than cycling a state, and it
        // reaches no row of this page.
        (Some("program"), "space", &["Solo a region"]),
        // ------------------------------------------------------------------
        // The Inspector's grammar
        // ------------------------------------------------------------------
        // Three deep, and the deepest bay on the panel: `1 2 3` is the first
        // pane's second thing's third control.
        (Some("inspector"), DIGIT, &[]),
        // **`↑↓` on the anchor scrub a quarter beat, and on a parameter row
        // write it** — a tenth of what the control publishes.
        (
            Some("inspector"),
            "arrows",
            &["Scrub a deck a quarter beat", "Write a parameter"],
        ),
        // **`space` on the four chips**, each naming the state it arrives at
        // rather than a flip, which is the chips' own rule (P-0090).
        (
            Some("inspector"),
            "space",
            &[
                "Set a deck's sync mode",
                "Composite a deck's renderers",
                "Choose which renderer of a deck is live",
                "Set a node's authority",
            ],
        ),
        // **`enter` on a parameter row takes the attachment back**, which is
        // the act of the control the row draws: a parameter with nothing
        // holding it draws no sensitivity row at all.
        (Some("inspector"), "enter", &["Take a parameter back"]),
        // ------------------------------------------------------------------
        // The Mixer's grammar
        // ------------------------------------------------------------------
        // **A digit names the nth strip, and naming a strip is the deck
        // selection** — which is why that row keeps a key badge rather than
        // losing one.
        (Some("mixer"), DIGIT, &["Select a deck"]),
        // **The arrows walk the strips and step the two levels**, which is the
        // one entry where the same key reaches a row two ways: `&larr;&rarr;`
        // on the row of strips is the selection, and `&uarr;&darr;` on an
        // addressed trim or fader is a tenth. See [`SPELLED`] for what that
        // costs the check.
        (
            Some("mixer"),
            "arrows",
            &["Select a deck", "Gain", "Opacity"],
        ),
        // **`space` is the whole of this bay's five controls and three of its
        // head's.** The transition row is the head's (ADR-0343): the settings
        // decide what the next move means wherever it lands, which is what a
        // head is for.
        (
            Some("mixer"),
            "space",
            &[
                "Put a deck on air, prime it, or take it off",
                "Gain",
                "Opacity",
                "Blend mode",
                "Set a deck's mask shape",
                "Choose the wipe shape, the quantum, the length",
            ],
        ),
        // **`enter` on the head's `go` capsule runs the transition on the
        // addressed strip**, which is the deck selection: this deck is covered
        // and the next one round arrives over it. **Only the wipe**, because
        // the row draws one capsule and `Operation::Wipe` is what it asks for
        // — the fade and the crossfade have no control on this row and their
        // panel badges say so.
        (Some("mixer"), "enter", &["Wipe the next deck in"]),
        // ------------------------------------------------------------------
        // The Master chain's grammar
        // ------------------------------------------------------------------
        // **The three effects have no addressable controls**, because
        // `Operation::SetFeedback { params: Undecided }` and its two
        // neighbours carry no spelling for a parameter — so a digit reaches
        // the effect and stops, which is ADR-0259's one place where a bay is
        // drawn and its operations are not sayable.
        (Some("master"), DIGIT, &[]),
        (Some("master"), "arrows", &["Master out"]),
        (Some("master"), "space", &["Master out"]),
        // ------------------------------------------------------------------
        // The Sequencer's grammar
        // ------------------------------------------------------------------
        // **Sixteen steps outrun ten digits**, so the digits reach a lane's
        // label and the first eight of its cells and the rest are walked —
        // which ADR-0259 calls *"honest and very nearly useless"*. Walking is
        // not an operation, so neither key names a row.
        (Some("sequencer"), DIGIT, &[]),
        (Some("sequencer"), "arrows", &[]),
        // **`space` is the whole of this bay**: the head's mode and bank
        // pills, a lane's label and a lane's cells, each named as the state it
        // arrives at. **`+ lane` is not here** — its press puts the chooser
        // down and what a lane is pointed at is picked in that card, which the
        // address does not descend into.
        (
            Some("sequencer"),
            "space",
            &[
                "Choose what a step is worth",
                "Choose which pattern the sequencer plays",
                "Mute a lane",
                "Toggle a step",
            ],
        ),
        // ------------------------------------------------------------------
        // The Outputs row's grammar
        // ------------------------------------------------------------------
        // **The simplest of the nine**: items are the sinks, each has exactly
        // one state, and `space` is the whole of it. The picture's on and off
        // is one operation and one fold, which is `Readout::sink`.
        (Some("outputs"), DIGIT, &[]),
        (Some("outputs"), "arrows", &[]),
        (Some("outputs"), "space", &["Choose where the frame goes"]),
    ];

    /// The routes that reach no row, so that one which starts reaching one
    /// stops being an exception, and a new exception is written down rather
    /// than discovered. The reasons are at the entries in [`ROWS`].
    ///
    /// A pair rather than a key, for [`ROWS`]' reason: `space` reaches six
    /// rows in the Mixer and two in the Library, and a list of *keys* that
    /// reach nothing could not say that.
    ///
    /// **In [`ROWS`]' own order**, which is what the check compares.
    const NO_ROW: &[(Option<&str>, &str)] = &[
        (None, "n"),
        (None, "esc"),
        (None, "tab"),
        (None, "backspace"),
        (Some("transport"), DIGIT),
        (Some("library"), DIGIT),
        (Some("library"), "arrows"),
        (Some("staging"), DIGIT),
        (Some("staging"), "arrows"),
        (Some("program"), DIGIT),
        (Some("program"), "arrows"),
        (Some("inspector"), DIGIT),
        (Some("master"), DIGIT),
        (Some("sequencer"), DIGIT),
        (Some("sequencer"), "arrows"),
        (Some("outputs"), DIGIT),
        (Some("outputs"), "arrows"),
    ];

    /// Byte for byte what `karakuri-console/tests/panel_column.rs` does, and
    /// resolves [`PAGE`] from it.
    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root")
    }

    fn page() -> String {
        let path = workspace().join(PAGE);
        fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "{} is the specification and could not be read: {e}",
                path.display()
            )
        })
    }

    /// **The grammar guard's own keys, beside the digit.**
    ///
    /// `crate::grammar` matches all six as literals — `Key::Named(NamedKey::…)`
    /// arms `bound` used to scan for — but they stay declared here rather
    /// than scanned for the same reason [`DIGIT`] always was one line down:
    /// [`super::KEY_BINDINGS`] is checked against
    /// `docs/manual/operations.html` directly by
    /// [`every_binding_the_table_names_a_title_for_reaches_a_route_marked_built`]
    /// below, and a scan of this file's text is not what answers *what does
    /// the window loop bind* for any key any more, table-driven or guard.
    const GRAMMAR_KEYS: &[&str] = &["up", "down", "left", "right", "space", "enter"];

    /// **`backspace`, which is neither in [`super::KEY_BINDINGS`] nor in
    /// `crate::grammar`.** It is a literal in the two letter-taking flows —
    /// the arrangement pill's name and the inspector's — that return before
    /// `window_event`'s own `match` on `key.logical_key` is ever reached, so
    /// it belongs to neither table. Both flows bind it unconditionally, so
    /// unlike [`DIGIT`] it is declared outright in [`bound`] rather than
    /// asked of anything at runtime.
    const NAME_ENTRY_KEY: &str = "backspace";

    /// **Every key the window loop binds, both the table and the grammar
    /// guard beside it — declared rather than scanned.**
    ///
    /// [`super::KEY_BINDINGS`] answers the ten literal keys directly: this
    /// is now a lookup over data `window_event` itself dispatches through,
    /// not a second copy of it. The other eight — `crate::grammar`'s four
    /// named keys, the four arrows, and the digit — are not in that table
    /// (`crate::grammar`'s own doc comment says why: it is a guard rather
    /// than arms, for the same reason the digits were always a special
    /// case here), so they are declared in [`GRAMMAR_KEYS`] and [`DIGIT`]
    /// rather than read out of this file's source. And `backspace` — see
    /// [`NAME_ENTRY_KEY`] — is neither the table's nor the grammar's, and is
    /// declared for its own reason beside them. None of the four is data
    /// this function could observe wrongly — every one names permanent
    /// code, not a configuration — so declaring them is not a weaker check
    /// than scanning for them was; it is the same facts, asserted instead
    /// of parsed.
    fn bound() -> BTreeSet<String> {
        let mut found: BTreeSet<String> = super::KEY_BINDINGS
            .iter()
            .map(|binding| binding.legend.to_owned())
            .collect();
        found.extend(GRAMMAR_KEYS.iter().map(|key| (*key).to_owned()));
        found.insert(NAME_ENTRY_KEY.to_owned());
        // **The one key of the grammar that names a different row in every
        // bay**, contributed by the console's dispatch table rather than
        // declared unconditionally like the rest of [`GRAMMAR_KEYS`] — see
        // [`DIGIT`]. `crate::grammar` binds it with a guard rather than a
        // literal, and *which* rows it reaches depends on
        // `karakuri_console::focus::BUILT` rather than on anything this
        // file says, so this is the one entry that still asks the console
        // rather than stating a fact `main.rs` alone could get wrong.
        if !karakuri_console::focus::BUILT.is_empty() {
            found.insert(DIGIT.to_owned());
        }
        found
    }

    /// **Every row's title and its key badge**, in page order: the badge's
    /// class — `has`, `plan` or `gap` — and the keys it names.
    ///
    /// Read verbatim and never decoded, which is `mcp.rs`'s rule and
    /// `panel_column.rs`'s after it: a badge that names nothing says `&mdash;`,
    /// and a key that needed decoding to match would be a key nobody could find
    /// on their keyboard.
    fn key_badges() -> Vec<(String, String, String)> {
        let html = page();
        let mut found = Vec::new();
        for row in html.split(ROW).skip(1) {
            let Some(open) = row.find("<h3>") else {
                continue;
            };
            let rest = &row[open + "<h3>".len()..];
            let Some(close) = rest.find("</h3>") else {
                continue;
            };
            let title = rest[..close].to_string();
            // The row ends where the next section does; a badge found past
            // that would belong to another row.
            let body = &rest[close..];
            let body = &body[..body.find("</section>").unwrap_or(body.len())];
            let mut badge = None;
            for span in body.split(r#"<span class="rt "#).skip(1) {
                let Some(quote) = span.find('"') else {
                    continue;
                };
                let class = span[..quote].to_string();
                let Some(text) = span[quote..].strip_prefix(r#"">key <b>"#) else {
                    continue;
                };
                let Some(shut) = text.find("</b>") else {
                    continue;
                };
                badge = Some((class, text[..shut].to_string()));
                break;
            }
            let Some((class, keys)) = badge else {
                continue;
            };
            found.push((title, class, keys));
        }
        found
    }

    /// The rows [`ROWS`] says a route reaches, or `None` if this program does
    /// not bind it at all.
    ///
    /// A route is a key **and** the bay it is addressed in, `None` for a
    /// global — which is the whole of what changed here: `space` alone names
    /// no route, and `("mixer", "space")` names five rows.
    fn rows_of(bay: Option<&str>, key: &str) -> Option<&'static [&'static str]> {
        ROWS.iter()
            .find(|(b, k, _)| *b == bay && *k == key)
            .map(|(_, _, rows)| *rows)
    }

    /// **What a key badge says, parsed** — the keys it names and the bay it
    /// names them in, or `None` for a badge that is not one of the two
    /// spellings this column carries.
    ///
    /// The two spellings are ADR-0331's: a **built** badge names bare letters,
    /// and a **designed** one names a key of the grammar and the bay it is
    /// addressed in. Since 2026-09-10 a built badge may be either, because the
    /// grammar is bound in two bays — which is the clause that record left for
    /// *"the code that binds `Tab`"* and this is it.
    ///
    /// A badge naming a bay resolves every key in it to that bay; a badge
    /// naming none resolves every key to a global. **A badge cannot mix them**,
    /// and that is not a limitation to work around: a press goes to the bay
    /// that has focus or it does not, and a row reached both ways would need
    /// two badges rather than one with two halves.
    fn parsed(badge: &str) -> Option<(Vec<String>, Option<&'static str>)> {
        let (keys, bay) = match badge.split_once(IN_THE) {
            Some((keys, bay)) => {
                let bay = bay.trim();
                // **The any-bay spelling first**, because it is not one of the
                // nine and resolving it against [`BAYS`] would answer `None`
                // and read as a badge nobody can parse.
                let bay = match bay == ANY_BAY.0 {
                    true => ANY_BAY.1,
                    false => BAYS
                        .iter()
                        .find(|(page, _)| *page == bay)
                        .map(|(_, name)| *name)?,
                };
                (keys, Some(bay))
            }
            None => (badge, None),
        };
        let keys = keys
            .split_whitespace()
            .map(|key| match key.chars().all(|c| c.is_ascii_digit()) {
                // Every digit is the one key of the grammar, which is what
                // makes `1 2 3 4` a badge naming one route and not four.
                true => DIGIT.to_owned(),
                false => SPELLED
                    .iter()
                    .find(|(page, _)| *page == key)
                    .map_or_else(|| key.to_owned(), |(_, name)| (*name).to_owned()),
            })
            .collect();
        Some((keys, bay))
    }

    /// The floor under both directions: a scan that matched nothing would
    /// satisfy every loop below by iterating over nothing at all.
    #[test]
    fn the_scan_finds_the_page_and_the_keys() {
        let badges = key_badges();
        assert!(
            badges.len() >= 54,
            "only {} rows with a key badge found in {PAGE} — is a row still `{ROW}` followed by \
             an `<h3>` and its `rt` badges?",
            badges.len()
        );
        assert!(
            bound().len() >= 9,
            "only {} keys found bound — the window loop's `match` has more arms than \
             this, and a check below it is a check that has stopped matching code",
            bound().len()
        );
    }

    /// **The list, the `match` and the legend are one list.**
    ///
    /// [`crate::KEYS`] is the table the window loop prints when it starts, and
    /// it is the one thing here the compiler cannot check: an arm added
    /// without an entry — or an entry left behind by an arm that went —
    /// arrives as a failure rather than as a key nobody noticed had stopped
    /// being reachable, *or as a legend that goes on telling an operator this
    /// program folds, solos, resets and quits*.
    ///
    /// That second half is why the printed table is the checked one. It was a
    /// separate list of nine `println!`s, and it stayed at nine while ten more
    /// keys were bound: the maintainer who read it reported the program
    /// unchanged, which it was not.
    #[test]
    fn the_keys_this_file_lists_are_the_keys_the_window_loop_binds() {
        let listed: BTreeSet<String> = KEYS.iter().map(|(k, _)| (*k).to_owned()).collect();
        assert_eq!(
            bound(),
            listed,
            "the keys the `match` in `window_event` binds are not the ones `KEYS` lists — which \
             is the list the legend prints. An arm this file does not know about reaches an \
             operation nothing checks the badge of and is told to nobody; an entry with no arm \
             is a legend naming a key an operator presses to no effect"
        );
    }

    /// **[`super::KEY_BINDINGS`] checked directly against the page**, which
    /// is what a table buys that a text scan never could: a key that names
    /// a row can be held against that row, rather than merely counted.
    /// [`ROWS`] checks the same page for the same ten keys already, by
    /// hand, in [`every_key_the_instrument_binds_reaches_a_route_marked_built`]
    /// below — this is the table checking itself, off `KeyBinding::title`
    /// rather than off a second, hand-written list, and it is what makes
    /// `title` a fact this file relies on rather than a field nothing reads.
    #[test]
    fn every_binding_the_table_names_a_title_for_reaches_a_route_marked_built() {
        let badges = key_badges();
        for binding in super::KEY_BINDINGS {
            let Some(title) = binding.title else {
                continue;
            };
            let found = badges
                .iter()
                .find(|(page_title, _, _)| page_title == title)
                .unwrap_or_else(|| {
                    panic!(
                        "`{}` names `{title}` and {PAGE} has no row with that heading — the \
                         page is the specification, so add the row there first",
                        binding.legend
                    )
                });
            assert_eq!(
                found.1,
                "has",
                "`{}`{} performs `{title}` at the panel, which {PAGE} marks `{}` in the key \
                 column — an operation an operator reaches from the keyboard and a page that \
                 says the instrument does not",
                binding.legend,
                said(binding.bay),
                found.1
            );
            let (keys, named) = parsed(&found.2).unwrap_or_else(|| {
                panic!(
                    "{PAGE} marks `{title}` built in the key column and its badge `{}` names a \
                     bay this file has no name for",
                    found.2
                )
            });
            assert_eq!(
                named,
                binding.bay,
                "`{}`{} performs `{title}` and {PAGE}'s badge for it reads `{}`{} — a press \
                 goes to the bay that has focus, so a badge naming the wrong bay tells an \
                 operator to address the key somewhere the press does nothing",
                binding.legend,
                said(binding.bay),
                found.2,
                said(named)
            );
            assert!(
                keys.iter().any(|k| k == binding.legend),
                "`{}`{} performs `{title}` and {PAGE} marks that row built in the key column \
                 naming `{}` — a badge that says an operator reaches it by pressing something \
                 else",
                binding.legend,
                said(binding.bay),
                found.2
            );
        }
    }

    // **The grammar's mix answers act on the deck the address is on, and read
    // the value they step from off that deck** — no longer checked here.
    //
    // Until 2026-09-11 this was a `#[test]`,
    // `the_grammars_mix_answers_act_on_the_addressed_deck_and_read_it_off_the_deck`,
    // that read this file as text and looked for the statements below as
    // literal substrings. It caught the same three claims a run of this
    // program actually makes, at the cost every scan in this module pays:
    // adding a comment that happened to contain one of these strings, or
    // reformatting `holding` so a blank line no longer flattened the way the
    // cut expected, failed the test for a reason that had nothing to do with
    // any of the three claims.
    //
    // - **The deck is the one the address named**, and **the reading is the
    //   deck's, not a strip's** — both of [`holding`]'s claims — are now
    //   `gpu::holding_reads_the_addressed_decks_own_state_and_never_a_strip_that_predates_it`,
    //   which presses [`holding`] itself against a deck moved after a frame
    //   had already copied its old state, and checks the *values* it hands
    //   back rather than the syntax it is spelled with.
    //   `gpu::a_mix_key_moves_the_deck_operator_selected_and_leaves_the_others_alone`
    //   presses the same two claims for [`gain_key`] and [`opacity_key`], the
    //   pair [`answered`] steps for the trim and the fader, the same way.
    // - **Nothing here reaches for a strip because nothing here has one to
    //   reach for**, which used to be the scan's fourth assertion and is now
    //   a fact about the crate graph rather than about this file's text:
    //   [`holding`]'s only parameters are `&Deck` and a slot, and
    //   `karakuri-engine` does not depend on `karakuri-console` (ADR-0156),
    //   so there is no `view::Strip` a function with that signature could
    //   name even by mistake. A scan cannot make that claim stronger than the
    //   crate graph already does, and does not need to try.
    // - **[`held`] guards every read**, and **`answered` builds
    //   `Operation::SetGain`/`SetOpacity` naming the addressed deck**, are
    //   the two claims this file cannot re-derive behaviourally: [`answered`]
    //   takes `&mut Gfx`, which bundles a live `winit::window::Window` and a
    //   `wgpu::Surface`, and nothing in this workspace builds one off-screen
    //   for a test the way [`Engine`] is built for [`Deck`]-only checks.
    //   `gpu::a_mix_key_moves_the_deck_operator_selected_and_leaves_the_others_alone`
    //   presses [`held`], [`gain_key`] and [`opacity_key`] by hand, in the
    //   same order [`answered`]'s `Trim`/`Fader` arm calls them, and is the
    //   nearest a test in this crate gets to entering [`answered`] itself —
    //   its own doc says so. The master out, the exposure and the latency
    //   offset arms, and the two arms that are not operations at all
    //   (`focus::Asked::Panel` and `Routed`, which leave through
    //   `Readout::op` and `Readout::sink`), are checked only by their own
    //   pure functions' tests (`the_master_out_steps_…`,
    //   `the_exposure_steps_…`, `the_offset_steps_…`) and by
    //   `karakuri-console`'s own tests of what `Readout::op` and
    //   `Readout::sink` do once called — not by anything that presses
    //   [`answered`] and watches those five arms run. That gap predates this
    //   change: the retired scan read the same five arms' text and could only
    //   ever say they were *spelled*, never that they ran, so nothing here
    //   is weaker for their sake than it was.

    /// **And every key the legend prints has its rows written down**, both
    /// ways round, which is what keeps [`ROWS`] from being a second list of
    /// keys rather than a mapping off the first.
    #[test]
    fn every_key_the_legend_prints_has_its_rows_written_down() {
        let printed: BTreeSet<&str> = KEYS
            .iter()
            .map(|(k, _)| *k)
            // **The four arrow keys are one route**, which is the one place
            // the legend and the routes count differently and it is the
            // grammar's own shape: `up`, `down`, `left` and `right` are four
            // keys an operator presses and *the arrows* is one rule about
            // kinds of thing. The legend prints four sentences; [`ROWS`] holds
            // one entry per bay.
            .map(|key| match SPELLED.iter().find(|(page, _)| *page == key) {
                Some((_, name)) => *name,
                None => key,
            })
            .collect();
        let mapped: BTreeSet<&str> = ROWS.iter().map(|(_, k, _)| *k).collect();
        assert_eq!(
            printed, mapped,
            "a key the legend prints has no entry in `ROWS`, or `ROWS` maps a key the legend \
             does not print. The rows a key reaches cannot be derived — a fold is a bay or a \
             pane depending on the pointer — so the mapping is written down, and this is what \
             says it is written down for exactly the keys this program binds"
        );
    }

    /// And the keys that reach no row are exactly [`NO_ROW`], both ways round.
    #[test]
    fn the_keys_that_reach_no_row_are_the_ones_written_down() {
        let silent: Vec<(Option<&str>, &str)> = ROWS
            .iter()
            .filter(|(_, _, rows)| rows.is_empty())
            .map(|(bay, key, _)| (*bay, *key))
            .collect();
        assert_eq!(
            silent, NO_ROW,
            "the keys that reach no row on {PAGE} are not the ones this file says they are — a \
             key that performs something the page never specified is a route nobody named"
        );
    }

    /// **A route reaching past the page.**
    ///
    /// A route this program binds whose row is not marked built in the key
    /// column — ADR-0213's failure mode from the side where the code moved
    /// first, which is how this whole column came to be wrong: the panel binary
    /// was given six arrangement keys and six rows went on reading `gap`.
    ///
    /// **The badge is parsed rather than word-matched** since 2026-09-10, which
    /// is the rewrite ADR-0259 scheduled: a key alone no longer determines a
    /// row, so the badge has to be read as *these keys, in that bay* and
    /// resolved against the pair.
    #[test]
    fn every_key_the_instrument_binds_reaches_a_route_marked_built() {
        let badges = key_badges();
        for (bay, key, rows) in ROWS {
            for row in *rows {
                let found = badges
                    .iter()
                    .find(|(title, _, _)| title == row)
                    .unwrap_or_else(|| {
                        panic!(
                            "`{key}` reaches `{row}` and {PAGE} has no row with that heading — \
                             the page is the specification, so add the row there first"
                        )
                    });
                assert_eq!(
                    found.1,
                    "has",
                    "`{key}`{} performs `{row}` at the panel, which {PAGE} marks `{}` in the key \
                     column — an operation an operator reaches from the keyboard and a page \
                     that says the instrument does not. Flip the badge, or say here why the key \
                     does not reach it",
                    said(*bay),
                    found.1
                );
                let (keys, named) = parsed(&found.2).unwrap_or_else(|| {
                    panic!(
                        "{PAGE} marks `{row}` built in the key column and its badge `{}` names a \
                         bay this file has no name for — a badge is a key, or a key and the bay \
                         it is addressed in",
                        found.2
                    )
                });
                assert_eq!(
                    named,
                    *bay,
                    "`{key}`{} performs `{row}` and {PAGE}'s badge for it reads `{}`{} — a press \
                     goes to the bay that has focus, so a badge naming the wrong bay tells an \
                     operator to address the keys somewhere the press does nothing",
                    said(*bay),
                    found.2,
                    said(named)
                );
                // **A row is reached by one route on this page**, which is
                // what one badge per row comes to: a row reached both by a
                // letter and by the grammar would need two badges, and the
                // column has one. That is why `f`, `s` and `u` are unbound —
                // see [`crate::KEYS`], where each is named.
                assert!(
                    keys.iter().any(|k| k == key),
                    "`{key}`{} performs `{row}` and {PAGE} marks that row built in the key column \
                     naming `{}` — a badge that says an operator reaches it by pressing \
                     something else",
                    said(*bay),
                    found.2
                );
            }
        }
    }

    /// **The page claiming a route nothing binds.**
    ///
    /// It fails apart from the test above because it is the other failure: that
    /// one says the program reached past the specification, this one says the
    /// specification tells a player to press a key the instrument does not
    /// read. It is the likelier of the two here, because twenty rows carried a
    /// built badge for `karakuri-cli`'s keyboard before the column said whose
    /// it was.
    #[test]
    fn every_key_route_the_page_marks_built_is_bound_by_the_instrument() {
        let badges = key_badges();
        let claimed: Vec<&(String, String, String)> = badges
            .iter()
            .filter(|(_, class, _)| class == "has")
            .collect();
        assert!(
            claimed.len() >= 6,
            "only {} rows of {PAGE} mark a key route built — the scan found less than the column \
             holds, which would pass this test by finding nothing",
            claimed.len()
        );
        for (title, _, badge) in claimed {
            assert_ne!(
                badge, NOWHERE,
                "{PAGE} marks `{title}` built in the key column and names no key for it — a \
                 `has` badge says an operator reaches the operation, so it has to say what to \
                 press"
            );
            let (keys, bay) = parsed(badge).unwrap_or_else(|| {
                panic!(
                    "{PAGE} marks `{title}` built in the key column and its badge `{badge}` names \
                     a bay this file has no name for — the nine are in `BAYS`, spelled the way \
                     the page says them"
                )
            });
            for key in keys {
                // **`in any bay` is read as all nine**, which is the stronger
                // claim and the honest one: a badge that says a press works
                // wherever focus is has to be true wherever focus is. The
                // route is written down once, under `ANY`, and this is what
                // holds the page's *any* to the console's.
                if bay == Some(ANY) {
                    assert!(
                        karakuri_console::focus::BUILT.len() == BAYS.len(),
                        "{PAGE} says `{title}` is reached in any bay and \
                         `karakuri_console::focus::BUILT` declares {} of the {} the arrangement \
                         has — a press that works in some of them is not one that works in any",
                        karakuri_console::focus::BUILT.len(),
                        BAYS.len()
                    );
                }
                let rows = rows_of(bay, &key).unwrap_or_else(|| {
                    panic!(
                        "{PAGE} marks `{title}` built in the key column and names `{key}`{}, \
                         which this program does not bind — the page tells a player to press a \
                         key the instrument does not read. Either the key went and the badge is \
                         `plan` again, or it is another program's: the key column is the \
                         instrument's keyboard, and `karakuri-cli`'s keys are its own",
                        said(bay)
                    )
                });
                assert!(
                    rows.contains(&title.as_str()),
                    "{PAGE} marks `{title}` built in the key column and names `{key}`{}, which \
                     this program binds to {rows:?} instead — one press, two operations",
                    said(bay)
                );
            }
        }
    }

    /// How a message names the bay a route is addressed in, or says it is
    /// global.
    fn said(bay: Option<&str>) -> String {
        match bay {
            Some(bay) if bay == ANY => String::from(" in any bay"),
            Some(bay) => format!(" in the {bay}"),
            None => String::from(" globally"),
        }
    }

    /// **The grammar the page is checked against is the grammar the console
    /// declares**, both ways round.
    ///
    /// This is the half ADR-0259 asked for and ADR-0331 could not have:
    /// *"`key_column`'s machinery changes shape … the check reads this file's
    /// own `match` arms as text, so a keyboard that becomes a per-bay dispatch
    /// table is invisible to it — which is the thing to solve rather than to
    /// discover."*
    ///
    /// [`ROWS`] holds the rows because a page heading is what a check reads and
    /// is not something this program says to anybody.
    /// `karakuri_console::focus::reaches` holds the **pairs**, because which
    /// keys act in which bay is a property of the dispatch and not of this
    /// file's text. Neither is derivable from the other, and this is what keeps
    /// them from being two answers:
    ///
    /// - **A bay whose grammar is built and has no rows written down** is a
    ///   press an operator can make that no badge on the page describes.
    /// - **A pair written down that the console does not declare** is a badge
    ///   telling an operator to press a key in a bay where nothing dispatches
    ///   it, which is exactly the failure a built badge naming a bay was
    ///   forbidden to make until now.
    #[test]
    fn the_grammar_the_page_names_is_the_grammar_the_console_declares() {
        let declared: BTreeSet<(&str, &str)> = karakuri_console::focus::reaches()
            .into_iter()
            .map(|(bay, key)| {
                (
                    bay,
                    match key {
                        karakuri_console::focus::Grammar::Digit => DIGIT,
                        karakuri_console::focus::Grammar::Arrows => "arrows",
                        karakuri_console::focus::Grammar::Space => "space",
                        karakuri_console::focus::Grammar::Enter => "enter",
                    },
                )
            })
            .collect();
        let written: BTreeSet<(&str, &str)> = ROWS
            .iter()
            // **The four keys of the grammar and not every route with a bay in
            // it.** `g` is addressed to the focused bay too — its operand is
            // the bay that has focus — but it is a letter this file binds and
            // not one of the four the console dispatches, so the console
            // declares nothing about it and holding it against that table
            // would be asking the wrong half.
            .filter(|(_, key, _)| [DIGIT, "arrows", "space", "enter"].contains(key))
            .filter_map(|(bay, key, _)| bay.map(|bay| (bay, *key)))
            .collect();
        assert_eq!(
            written, declared,
            "the routes this file writes down and the ones `karakuri_console::focus::BUILT` \
             declares are not the same routes. A pair the console declares and this file does \
             not is a press an operator can make that no badge describes; a pair here the \
             console does not declare is a badge naming a key that reaches nothing in that bay"
        );
        assert!(
            declared.len() >= 31,
            "only {} routes are declared by the dispatch table — nine bays and the fold that is \
             addressed in all of them come to 31, so a scan finding fewer has stopped reading it",
            declared.len()
        );
    }

    /// **Every bay a badge names is a bay the arrangement has**, which is the
    /// floor under [`parsed`]: a spelling nobody can resolve reads as a global
    /// key, and a global key that reached a bay's row would pass both badge
    /// checks by naming the wrong thing consistently.
    #[test]
    fn the_bay_names_the_page_uses_are_the_arrangements_own() {
        for (page, name) in BAYS {
            assert!(
                karakuri_console::view::region(name).is_some(),
                "{PAGE} names a bay `{page}` and this file resolves it to `{name}`, which is not \
                 a region the console draws"
            );
        }
        assert_eq!(
            BAYS.len(),
            9,
            "the manual's *What each region is standing on* lists nine bays and this file has {}",
            BAYS.len()
        );
    }
}
