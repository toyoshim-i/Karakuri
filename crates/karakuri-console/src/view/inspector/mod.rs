use super::*;

pub mod header;
pub mod params;
pub mod wiring;

pub use header::*;
pub use params::*;
pub use wiring::*;

pub(crate) use header::next_sync;

// ---------------------------------------------------------------------------
// The Inspector
// ---------------------------------------------------------------------------

/// How many panes the inspector has, which is `lib.rs`'s [`arrangement`] and
/// the mock's `.insp-split` read as one number: the CSS is
/// `grid-template-columns: 1fr 9px 1fr`, two tracks and the bar between them,
/// and the arrangement builds `inspector-1` and `inspector-2` to match.
///
/// A [`Spec`](karakuri_layout::Spec) builds a
/// [`Layout`](karakuri_layout::Layout) once and the arena has no insert, so the
/// count is settled at build time — the mock's `2 up ▾` is an operator choosing
/// it while running, and that is a control this pass does not add.
pub const PANES: usize = 2;

/// The arrangement's name for each pane, in the order the mock draws them.
///
/// Public for [`DECK_LETTERS`]'s reason: a harness that says *which pane* has
/// to say it in the names the arrangement addresses them by, and a second list
/// written out there would go on saying `inspector-1` the day this one does
/// not.
pub const PANE_NAMES: [&str; PANES] = ["inspector-1", "inspector-2"];

/// Which deck each pane opens pointed at — the first pane at deck A and the
/// second at deck B, which is the mock's own two heads.
///
/// It is what this console did before the pulldown existed, written down as a
/// *default* rather than left as the host's habit: the panes used to be filled
/// slot by slot, so a fourth slot could not be looked at at all. See
/// [`View::pane_deck`], which is the pointer this seeds.
///
/// Not a reading of anything, so a console with fewer strips than this names
/// opens with a pane pointed at a deck the mixer draws none for — which is a
/// pane with no nodes in it and is the honest state, exactly as
/// [`View::selection`]'s deck A is on a console with no deck behind it.
///
/// [`View::pane_deck`]: View::pane_deck
pub const PANE_DECKS: [u8; PANES] = [0, 1];

/// What one pane of the inspector is showing, handed in by whoever has a deck —
/// the same seam [`Strip`] crosses, one bay along.
///
/// `src/` takes no device and no engine (ADR-0156), so nothing here asks a
/// `Set` anything: every field is a value somebody who *can* ask read off one
/// and wrote down. `crates/karakuri` is where that reading is, and it is where
/// the two omissions below are decided as well.
#[derive(Debug, Clone, PartialEq)]
pub struct Pane {
    /// Which deck this pane is pointed at, as an index into [`DECK_LETTERS`] — the
    /// mock's `deck A`.
    ///
    /// The pane is pointed rather than choosing, and that is the mock's `showing …
    /// ▾` not being drawn: the chooser is a control and this pass adds none, so
    /// whoever fills this says which deck each pane shows.
    ///
    /// And it is not [`View::selection`], which this console does now keep. That is
    /// *the* deck — one value, what a key press is addressed to, drawn as one ring
    /// — and there are two panes: a chooser here picks a deck to *look at* while
    /// the keys stay where they were, which is the whole of why the mock draws a
    /// caret in each pane head and a ring on one strip. So this waits on a per-pane
    /// pointer nothing keeps, and reading the deck selection into it would fold two
    /// facts into one and make the second pane a copy of the first.
    pub deck: usize,
    /// What that deck is playing, which is the same name the deck's mixer strip
    /// carries and comes from the same place — see [`Strip::name`], and the short
    /// of it is that a `Set` has no name of its own and only whoever built it knows
    /// what to call it.
    pub material: String,
    /// What this deck's clock is locked to: `karakuri_engine::transport::Sync` as
    /// the vocabulary's copy of the same three.
    pub sync: Sync,
    /// Which of [`SYNCS`] this deck's material can honour, in that order — what the
    /// sync chip's cycle skips over.
    ///
    /// The answer and not the two facts it is computed from, which is the seam
    /// every other field here crosses read one step further along.
    /// `karakuri_engine::deck::Deck::sync_allowed` is what decides it, off whether
    /// the Set in the slot is closed form and whether it reads `beats`, and `src/`
    /// has no engine (ADR-0156) — so whoever owns one asks it three times and
    /// writes the three answers here. Handing in the two properties instead would
    /// put a third copy of `Transport::allows`' rule in a crate that owns no
    /// material, and a control is not the authority on what it may ask for
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// A property of the Set, so it moves when a build lands in the slot — the
    /// engine says so at `sync_allowed`, and it is why this is read beside
    /// [`Pane::sync`] rather than once.
    ///
    /// All three `true` is the whole of *nothing is refused*, which is what a
    /// console with no engine behind it and every test in this crate that does not
    /// say otherwise hands in; `Free` is refused by no material at all, so the
    /// first entry is never `false` in a reading anything took.
    pub allows: [bool; SYNCS.len()],
    /// The tempo the deck was engaged at, which is what its rate is measured
    /// against. Drawn only under [`Sync::Tempo`] and [`Sync::Beat`] — see
    /// [`anchor_letter`].
    pub anchor_bpm: f32,
    /// The scrub's own value, in beats, signed. Drawn only under [`Sync::Beat`],
    /// *"since that is the only mode that reads the offset"*.
    ///
    /// In beats and one deck's, where the transport row's offset is in milliseconds
    /// and is the whole instrument's — `style.css` says the unit is what tells them
    /// apart, *"so neither is ever drawn without one"*, and the mock's own `+0.25`
    /// is what this is drawn as.
    pub scrub_beats: f64,
    /// Whether this deck's Set folds its renderers into one result or overdraws
    /// them: `karakuri_engine::set::Layering`, as a bit.
    ///
    /// What the deck is doing, and what a press names the other of. The chip's
    /// tooltip is *"Click to overdraw them instead"*, and this is both the word
    /// [`deck_head_into`] draws and the state [`DeckHead::composite`] reads to say
    /// which layering a press is asking for — a destination and never a flip
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// It said the press was not a control until 2026-09-09, on the grounds that
    /// layering is a *build* decision in the engine — `Set::layering` answers off
    /// whether the Set was built with a merge, and nothing writes it afterwards.
    /// Both halves of that are still true and the conclusion was wrong: a rebuild
    /// is what this instrument already does to change what a slot is running, and
    /// the layering is one field of the aim a watcher is pointed at, so a press
    /// re-aims the slot and the worker rebuilds it — the route a library load
    /// takes, judged against the budget like any other build
    /// ([ADR-0314](../../../../docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
    /// It is a readout of what landed rather than of what was asked for, which is
    /// `Mixer::residency`'s division: the build may still be rolled back, and the
    /// Staging lane is what says so.
    pub composite: bool,
    /// The two fields of this slot's aim the deck head can move, or `None` on a
    /// deck with no geometry to size and no randomness to seed — see [`Aimed`],
    /// which is where the argument is.
    pub aimed: Option<Aimed>,
    /// The node groups, in node order, which is the order a Set addresses its own
    /// nodes in.
    pub nodes: Vec<Node>,
}

/// The Inspector's pane, laid out: the head that says which deck, the deck
/// head under it, and what is left for the node groups.
///
/// # What is in the mock's pane and is deliberately not here
///
/// This is [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
/// applied to the bay it named as the next one and the hardest: *draw every
/// part of the mock that has a value behind it and omit the rest outright — no
/// placeholder, and no empty case the mock did not itself draw.* Nine things
/// are omitted and each one is named with what it waits on. Three of the
/// nine have since been drawn, and they are the deck head's — see
/// [`DeckHead`], which is where their argument now lives.
///
/// Two are controls and are still not here.
///
/// - `showing … ▾`, the chooser in the pane head. Which deck a pane shows
///   is a per-pane pointer this console does not keep, and it is not the
///   deck selection: that one is what a key press is addressed to and there is
///   one of it, where there is a caret in every pane head — see
///   [`Pane::deck`]. So the pane is *pointed* by whoever fills that field and
///   the caret is not drawn. The word `showing` and the deck it names are a
///   readout and are.
/// - `keep`, the pill beside it: *"Keep deck A as a Set, exactly as it is
///   on screen … It goes into the library under a name."* That is a write into
///   the store, which is the Library bay's `load` from the other end —
///   and it is the end that is still open. A load re-points a slot's source
///   and lets the worker build it; a keep has to read a *running* Set back out
///   and name it, which is `Set::published`'s side of the seam and a different
///   question entirely.
///
/// A third was `composite`, called a readout here until 2026-09-09, and it
/// is a control — [`Pane::composite`] and [`DeckHead::composite`]. The
/// sentence that stood here said layering is a build decision in the engine so
/// a press on it is a rebuild rather than a write, and *"there is no operation
/// in the vocabulary for it to name"*, which was wrong twice:
/// `Operation::SetCompositing` has been in the vocabulary the whole time, and a
/// rebuild is exactly how this instrument changes what a slot is running. A
/// press re-aims the slot — the layering is one field of `watch::Aim` — and the
/// worker builds it off the render thread, which is the route a library load
/// takes
/// ([ADR-0314](../../../../docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
///
/// And the fourth was the anchor, which the mock draws as a readout and
/// [ADR-0218](../../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)
/// made a control. A press on it emits `SetSync` naming the mode the deck is
/// already in, which re-anchors, and its face goes on being a reading of two
/// numbers the deck has. The sync chip beside it and the scrub's two arrows
/// landed with it — the whole of the deck head is [`deck_head`] now, and this
/// type is the pane's three rows.
///
/// Two had no value in this workspace at all until 2026-09-09, which was
/// ADR-0191's rule — a panel drawing a state the engine never entered is a
/// drawing of one — and both are drawn now, because a press can attach a
/// signal (ADR-0319).
///
/// - `.param.bound`'s `.pval.src`, a bound parameter showing its source
///   instead of a number. [`Param::bound`] is the reading, off
///   `Set::bindings`, and what made it enterable is `Deck::bind` rather than
///   anything in this crate. The mock's other two sources are still states
///   this program cannot enter: `midi 21` and `seq 1` are not bindings at
///   all — no MIDI map reaches a Set's parameter, and a sequencer lane is a
///   fifth route into the vocabulary rather than a signal on the bus
///   (ADR-0222) — so a row drawn from either would be ADR-0191's drawing.
/// - The `.sens` row under a bound parameter — the signal, the curve, the
///   range and `take back`, which is [`SensChip`]. Two of its four are
///   controls and two are readouts, and the mock's `step` curve beside `seq 1`
///   is not one of the four this vocabulary has.
///
/// Two are the shape of the mock disagreeing with the shape of a Set, and
/// they are the two things this pass found:
///
/// - A published control that names no node has no row. The manual groups
///   parameters *"by node, the way a Set is addressed everywhere else"*, and
///   the mock draws every `.param` inside a `.node-group`. But
///   `Published::at` is an `Option` and the default interface — the one
///   every Set in `crates/karakuri` has, since that binary has no `--publish`
///   — is made entirely of wildcards: *"one control per key, not one per
///   declaration"*, addressed at every node that declares the key. A wildcard
///   covering exactly one node is that node's and is drawn there; one covering
///   several belongs to several groups and is omitted, because the mock
///   draws no row outside a group and inventing a place for one is a
///   specification written backwards. It waits on the page saying where such a
///   row goes.
/// - The `L4` group's authority chip. See [`Node::authority`].
///
/// And one is the pane running out of room, which is what
/// [`InspectorPane::scroll`] answers: the pane scrolls, and
/// [`InspectorPane::shown`] is what it says about that.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectorPane {
    /// `.half-head`, along the top of the pane, with its rule on the bottom.
    pub head: Rect,
    /// `.deck-head`, under it — the deck's own clock and its fold.
    pub deck_head: Rect,
    /// What is left under the two heads, where the node groups stack from the top
    /// with a hairline between them.
    pub body: Rect,
    /// How far this pane's body is scrolled, in force this frame — the stored
    /// position clamped against what there is to scroll through, and never the
    /// stored position itself.
    ///
    /// # Clamped here and stored nowhere
    ///
    /// The clamp is `0 ..= (content - body height)`, and both ends of that range
    /// move when the pane is dragged — so a clamp written back into [`View`] would
    /// be a resize rewriting what an operator scrolled to. That is
    /// [ADR-0250](../../../../docs/adr/0250-below-the-minima-the-arrangement-scales-rather-than-being-rewritten.md)'s
    /// rejected *clamp the stored size during the solve*, one region in, and
    /// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md) is
    /// the rule: a shorter pane draws less of the same position and stores nothing,
    /// so dragging it back reproduces what was on screen exactly rather than
    /// nearly.
    ///
    /// What [`View::scroll_by`] does clamp is the *content*, which is a reading of
    /// the deck rather than a viewport — see it for why the two are not the same
    /// clamp.
    pub scroll: f32,
    /// How tall everything in this pane is: [`group_h`] over every node with a
    /// [`size::HAIRLINE`] between two of them, whether or not any of it is on
    /// screen.
    ///
    /// It is the number [`scroll`](Self::scroll) is clamped against and the number
    /// [`View::scroll_by`] is clamped against, derived in one place so the two
    /// cannot disagree.
    pub content: f32,
    /// How many node groups this pane is showing whole, which is the `n` of the `n
    /// of m` its head reads — [`pane_count`], and rule 04 of [the
    /// manual](../../../../docs/manual/index.html): *"A list that showed you part
    /// of itself says so and says how much."*
    ///
    /// # It is the readout's number and not the walk's
    ///
    /// [`InspectorPane::drawn`] is what is painted and what a press is hit-tested
    /// against, and it is the wider of the two: a group cut by the top edge or the
    /// bottom one is drawn as far as the pane goes and can be pressed where it is
    /// drawn. This counts the ones that are whole, so that `m of m` means *nothing
    /// is out of sight* and can never be read off a pane with a group hanging over
    /// an edge.
    ///
    /// It used to be how many were drawn, and the two were one number. A pane drew
    /// a group whole or not at all, because there was no position to scroll to — so
    /// below roughly 534px of Inspector bay not one parameter row was drawn, in the
    /// bay whose whole content is parameter rows. The maintainer's answer to that
    /// was *the pane scrolls*, and this is the half of the old rule that survives
    /// it: a part-drawn group is no longer a lie about what a node has, because the
    /// count says how many are whole and the rest is one notch of the wheel away
    /// ([ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
    ///
    /// Zero is a state and not a `None`. A pane too short to hold one group whole
    /// still says which deck it is showing and what that deck's clock is doing, and
    /// still draws as much of the group as it has room for; it is [`inspector`]'s
    /// `None` that means *there is no pane here to draw*.
    pub shown: usize,
}

impl InspectorPane {
    /// Where the `index`th group goes, and how tall it is — in the pane's own
    /// coordinates, with [`scroll`](Self::scroll) already taken off, so a group
    /// above the body has a negative-going top and one below it a top past
    /// `body.max.y`.
    ///
    /// Derived rather than stored for [`LibraryBay::row`]'s reason — the groups are
    /// a walk and a `Vec` of rectangles would be an allocation a frame does not
    /// need — but a walk rather than a stride, because a group is as tall as what
    /// is in it.
    ///
    /// Every index in `nodes` is an answer, where this used to refuse anything past
    /// `shown`: a scrolled pane has groups off both edges and
    /// [`drawn`](Self::drawn) is what says which of them reach the picture, so the
    /// rectangle has to exist before that question can be asked. Past the end of
    /// `nodes` is still a caller's error.
    pub fn group(&self, nodes: &[Node], index: usize) -> Rect {
        let top = self.body.min.y - self.scroll
            + nodes
                .iter()
                .take(index)
                .map(|node| group_h(node) + size::HAIRLINE)
                .sum::<f32>();
        Rect::from_min_size(
            Pos2::new(self.body.min.x, top),
            egui::vec2(self.body.width(), group_h(&nodes[index])),
        )
    }

    /// Which groups reach the picture, as a range into `nodes` — the ones a
    /// scrolled body has any of on screen, cut edges included.
    ///
    /// It is what [`inspector_into`] paints and what [`InspectorPane::grip`] and
    /// [`InspectorPane::select_renderer`] walk, so a control is hit-tested over
    /// exactly the groups that were drawn. It is not [`shown`](Self::shown), which
    /// counts the whole ones and is the readout's number: a fader in a group cut by
    /// the bottom edge is drawn and is pressable, and the group it is in is not
    /// counted as shown.
    ///
    /// A walk rather than arithmetic, for [`group`](Self::group)'s reason: the
    /// groups are of unequal height. Empty where the pane's body has no height at
    /// all, which is a folded pane.
    pub fn drawn(&self, nodes: &[Node]) -> std::ops::Range<usize> {
        let mut first = nodes.len();
        let mut last = 0;
        let mut top = self.body.min.y - self.scroll;
        for (index, node) in nodes.iter().enumerate() {
            let bottom = top + group_h(node);
            if bottom > self.body.min.y && top < self.body.max.y {
                first = first.min(index);
                last = index + 1;
            }
            top = bottom + size::HAIRLINE;
        }
        match first < last {
            true => first..last,
            false => 0..0,
        }
    }

    /// What a press at `p` on a renderer chip asks for, or `None` off every chip
    /// this pane drew.
    ///
    /// # The row is a choice only where the deck composites and holds two
    ///
    /// [Every operation](../../../../docs/manual/operations.html) states the
    /// condition on the row itself — *"Only where the deck composites and holds two
    /// or more. One-way: no position in the cycle folds them all back in"* — and
    /// `docs/manual/console.html` states it from the chips' side: *"This Set
    /// composites, so one renderer is live and the rest are not."* Under overdraw
    /// every renderer draws, so a selection would name a state the picture is not
    /// in; with one renderer there is nothing to choose between. Both are drawn and
    /// neither is claimed, which is [`DeckHead::arrow`]'s arrangement on an inert
    /// scrub and this crate's own rule stated at [`crate::input`]: *a control
    /// claims what it acts on and no more*. The chips keep their shape either way,
    /// because a row that vanished when a deck stopped compositing would move every
    /// parameter row under it out from under the hand.
    ///
    /// Every chip of a live row is claimed, the lit one included. It names a
    /// destination, which is what an operation on this panel is
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and pressing the lit one is the selection the deck already has asked for
    /// again — the anchor's shape two rows up. A chip that stopped being pressable
    /// the moment it lit would take the claim out from under a hand on the beat the
    /// swap landed.
    ///
    /// # What is asked before what
    ///
    /// The body, then the group's row, then the chips — [`LibraryBay::chip`]'s
    /// order one bay along and for its reason: the chips are laid end to end from
    /// the row's left padding and [`inspector_into`] clips the paint to the pane,
    /// so a chip that finishes outside the pane is a target only for the part of it
    /// that is drawn. The row is the pane's own width, so that clip is this row's
    /// `contains`.
    ///
    /// It costs a galley lookup per chip and only inside a renderer row, which is
    /// [`LibraryBay::chips`]' price: a chip is as wide as the word in it, and the
    /// walk stops at the one under the pointer.
    pub fn select_renderer(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        // Fonts are not valid until `egui` has run a pass — [`deck_head`]'s
        // guard, and before the first one there is no chip drawn to press.
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            if !a_choice(pane, node) {
                return None;
            }
            let row = rend_row_in(self.group(&pane.nodes, index), node);
            if !row.contains(at) {
                return None;
            }
            rend_chips(ctx, row, &node.renderers)
                .find(|(_, chip)| chip.contains(at))
                .map(|(renderer, _)| Operation::SelectRenderer {
                    deck: pane.deck as u8,
                    renderer: renderer as u32,
                })
        })
    }

    /// What a press at `p` on a node head's `man / sug / auto` asks for, or `None`
    /// off every chip this pane drew.
    ///
    /// [`InspectorPane::select_renderer`]'s shape one row up, and the same order of
    /// questions: the body, then the group's head, then the chips. The head is the
    /// pane's own width, so the clip [`inspector_into`] paints under is this row's
    /// `contains`.
    ///
    /// A head with no chip is not a target, which is `Node::authority` being `None`
    /// — a head that folds more than one node has no one answer to draw and so no
    /// destination to press. Everything else about the walk is `select_renderer`'s.
    pub fn set_authority(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        // Fonts are not valid until `egui` has run a pass, and before the
        // first one there is no chip drawn to press — `deck_head`'s guard.
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let at_node = node.authority?.at;
            let group = self.group(&pane.nodes, index);
            let head = Rect::from_min_max(
                group.min,
                Pos2::new(group.max.x, group.min.y + size::NODE_HEAD_H),
            );
            if !head.contains(at) {
                return None;
            }
            // **Inside what the `keep` capsule leaves**, and the trim is
            // [`auth_chips`]' own rather than applied here — a press on the
            // capsule is [`InspectorPane::keep_procedure`]'s and reaches no
            // chip, because the chips are not drawn there.
            auth_chips(ctx, head, node)
                .find(|(_, chip)| chip.contains(at))
                .map(|(authority, _)| Operation::SetAuthority {
                    deck: pane.deck as u8,
                    node: at_node,
                    authority,
                })
        })
    }

    /// What a press at `p` on a node head's `keep` capsule asks for, or `None` off
    /// every capsule this pane drew.
    ///
    /// [`InspectorPane::set_authority`]'s walk at the other end of the same row,
    /// and the same order of questions: the body, the group's head, then the
    /// capsule.
    ///
    /// A head with no capsule is not a target, which is [`Node::keep`] being `None`
    /// — a head over several nodes, and the built-in camera. Both fall out here by
    /// the derivation answering `None` rather than by a check of their own, which
    /// is the same shape `set_authority` refuses a folded head in.
    ///
    /// `id: None`, and the store names the file. This is the press that types
    /// nothing, so it takes the stamp —
    /// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)'s
    /// two routes drawn on one capsule, exactly as [`KeepPill`] draws them for the
    /// deck. The name a head *has* typed is [`View::named_set`]'s, and the host is
    /// what pairs the two: a keep sent while this pane's head is asking for a name
    /// files under what was typed.
    pub fn keep_procedure(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let at_node = node.keep?;
            let group = self.group(&pane.nodes, index);
            let head = Rect::from_min_max(
                group.min,
                Pos2::new(group.max.x, group.min.y + size::NODE_HEAD_H),
            );
            if !head.contains(at) {
                return None;
            }
            node_keep(ctx, head, node)?
                .contains(at)
                .then_some(Operation::KeepProcedure {
                    deck: pane.deck as u8,
                    node: at_node,
                    id: None,
                })
        })
    }

    /// What a press at `p` on a sensitivity row's chips asks for, or `None` off
    /// every chip this pane drew and off the two that are readouts.
    ///
    /// [`InspectorPane::set_authority`]'s walk one level in: the body, the row,
    /// then the chips. A row nothing is holding has no sensitivity row at all —
    /// [`sens_rect`] answers `None` — so there is no case for a press on one, which
    /// is the mock's own arrangement rather than a check.
    ///
    /// The two readouts answer `None` here rather than being left out of
    /// [`sens_chips`], because they are drawn and a press has to be able to land on
    /// them and do nothing: leaving them out would put the chips after them in the
    /// wrong place.
    pub fn sensitivity(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        let deck = pane.deck as u8;
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let group = self.group(&pane.nodes, index);
            node.params.iter().enumerate().find_map(|(at_row, param)| {
                let source = param.bound.as_ref()?;
                let row = sens_rect(group, node, at_row)?;
                if !row.contains(at) {
                    return None;
                }
                sens_chips(ctx, row, source)
                    .find(|(_, chip)| chip.contains(at))
                    .and_then(|(chip, _)| chip.operation(deck, source))
            })
        })
    }

    /// What a press at `p` takes hold of in this pane, or `None` where there is
    /// nothing under it a hand can move.
    ///
    /// # It is the knob, and the track is deliberately not a target
    ///
    /// [`Mixer::grab`]'s rule and [`MasterRow::grab`]'s, and it is this bay's for
    /// the same reason read one bay along: a parameter at 0.2 whose track was
    /// clicked would put the value at the far end of its published range, on stage,
    /// because a hand landed three pixels off a knob. The mock draws a `.fader s`
    /// on every `.param` and deliberately draws none on the transport's exposure
    /// track, which is what tells a control with a handle from one that is set
    /// outright — *"a handle that jumped to the pointer would be a lie about what a
    /// handle is"*.
    ///
    /// The `.param` rows carry no tooltip in the mock, which is where the mixer's
    /// version of this rule is written down (*"a press on the track off the knob
    /// does nothing, which is every fader in this bay's rule"*), so the page does
    /// not yet say it for this bay. The console's answer is the mixer's;
    /// `docs/manual/console.html` is where it has to be said.
    ///
    /// # A row with nowhere to go is not taken hold of
    ///
    /// [`Param::movable`]: a published range of no width is a control with one
    /// position. The row is drawn — a fill at the start and a figure — and it is
    /// not a handle, which is `Grab::new`'s own refusal read on the value axis
    /// instead of on the track.
    ///
    /// # Only what is drawn, and only where it is drawn
    ///
    /// Two conditions, and they are two because the pane scrolls.
    /// [`drawn`](Self::drawn) is the groups that reach the picture, which is what a
    /// press may land in; `body.contains` is what keeps a row that has gone under a
    /// head from taking the press anyway. A group scrolled off the top still has a
    /// rectangle — [`group`](Self::group) answers one for every index — and that
    /// rectangle overlaps the deck head and the pane head above it, where the paint
    /// is clipped away and a knob is therefore not on screen. Without this check
    /// the pane would claim a press on a knob nobody can see, under a control that
    /// is drawn there; with it, a press outside the body reaches this bay's other
    /// derivations and no other, exactly as `select_renderer` beside it already
    /// asked.
    ///
    /// The clip is the authority and this is the same rectangle, which is
    /// [`inspector_into`]'s `with_clip_rect(at.body)` asked as a question rather
    /// than applied as a paint.
    pub fn grip<'a>(&self, pane: &'a Pane, p: karakuri_layout::Point) -> Option<ParamGrip<'a>> {
        let p = Pos2::new(p.x, p.y);
        if !self.body.contains(p) {
            return None;
        }
        // The manual's *deck* is the code's *slot*, and a deck holds
        // `MAX_SLOTS` of them — `DECKS`, which is 4 — so the index is a `u8`
        // with room to spare. [`Mixer::grab`]'s note, one bay along.
        let deck = pane.deck as u8;
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let group = self.group(&pane.nodes, index);
            node.params
                .iter()
                .enumerate()
                .find_map(|(at, param)| match param.movable() {
                    false => None,
                    true => {
                        let fader = param_fader(param_rect(group, node, at), param)?;
                        fader
                            .knob
                            .contains(p)
                            .then_some(ParamGrip { deck, param, fader })
                    }
                })
        })
    }

    /// Whether `p` is on a parameter fader's knob, which is what
    /// [`crate::input::claim`] asks — the knob, and not the track under it.
    pub fn owns(&self, pane: &Pane, p: karakuri_layout::Point) -> bool {
        self.grip(pane, p).is_some()
    }

    /// What a press at `p` on a parameter row's leftmost cell asks for:
    /// [`Operation::Publish`](karakuri_operation::Operation::Publish) carrying the
    /// interface this deck would have with that one control's membership changed —
    /// or `None` off every mark.
    ///
    /// # The whole list, because that is what the operation is about
    ///
    /// The vocabulary says it at the variant: *"the whole ordered list, not one
    /// entry. A MIDI control is bound to a position in the published interface, so
    /// adding one entry at a time would renumber every binding after it; and an
    /// interface that publishes nothing publishes everything, which is a statement
    /// about the list and not about an entry."* So this builds the list the press
    /// is asking for — every published row in interface order, less the one
    /// pressed, or with it appended where it was not on the list — and names it.
    /// Nothing here says *drop this one*, which is what keeps two hands on one deck
    /// from disagreeing about what is published
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// Sorted by [`Param::ord`] and not by where the rows are drawn. A wildcard
    /// control is *placed* in whichever group it resolves to and is *numbered* by
    /// its position in the interface, and the two orders are not the same walk — so
    /// building the list off the pane's own order would renumber every knob on a
    /// deck with a wildcard in it, on a press that was about a different row
    /// entirely.
    ///
    /// A row that goes back on lands at the end, which is a decision and not an
    /// accident: nothing in the pane says where it *was*, the position it left is
    /// now somebody else's, and inventing a place for it would move knobs nobody
    /// pressed anything about. The page says so.
    pub fn publishing(&self, pane: &Pane, p: karakuri_layout::Point) -> Option<Operation> {
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        let pressed = self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let group = self.group(&pane.nodes, index);
            node.params.iter().enumerate().find_map(|(row, param)| {
                ord_cell(param_rect(group, node, row), param)
                    .contains(at)
                    .then_some(param)
            })
        })?;
        let mut kept: Vec<(usize, &Param)> = pane
            .nodes
            .iter()
            .flat_map(|node| node.params.iter())
            .filter(|param| !std::ptr::eq(*param, pressed))
            .filter_map(|param| Some((param.ord?, param)))
            .collect();
        kept.sort_by_key(|(ord, _)| *ord);
        let mut controls: Vec<karakuri_operation::Control> =
            kept.into_iter().map(|(_, param)| param.control()).collect();
        if pressed.ord.is_none() {
            controls.push(pressed.control());
        }
        Some(Operation::Publish {
            deck: pane.deck as u8,
            controls,
        })
    }

    /// Where one row's publish mark is — the leftmost cell of the `row`th parameter
    /// row of the `node`th group, or `None` where the pane is not drawing that
    /// group or that row.
    ///
    /// The same rectangle [`InspectorPane::publishing`] resolves a press against
    /// and [`param_into`] paints into, which is this bay's rule everywhere: the
    /// derivation that draws a control is the one that hit-tests it.
    pub fn publish_mark(&self, pane: &Pane, node: usize, row: usize) -> Option<Rect> {
        if !self.drawn(&pane.nodes).any(|drawn| drawn == node) {
            return None;
        }
        let at = pane.nodes.get(node)?;
        let param = at.params.get(row)?;
        Some(ord_cell(
            param_rect(self.group(&pane.nodes, node), at, row),
            param,
        ))
    }

    /// Where one node's `index`th `uses` line's control is, or `None` where the
    /// pane is not drawing that group, that group has no such input, or the line
    /// falls outside the body.
    ///
    /// `open` is whether *this* line's card is down, which is the console's own
    /// state and not the pane's — [`View::wiring_open`], the arrangement [`Load`]
    /// is already in with [`View::target_open`].
    ///
    /// The capsule is as wide as the name in it, which is why this asks `egui` for
    /// a galley: a node's name is data and a capsule sized to a constant would clip
    /// one Set's names and not another's.
    pub fn uses_line(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        node: usize,
        index: usize,
        open: bool,
    ) -> Option<UsesLine> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = pane.nodes.get(node)?;
        let uses = at.uses.get(index)?;
        if !self.drawn(&pane.nodes).any(|drawn| drawn == node) {
            return None;
        }
        let row = uses_rect(self.group(&pane.nodes, node), index);
        if !self.body.contains_rect(row) {
            return None;
        }
        Some(UsesLine {
            chip: uses_chip_in(ctx, row, uses),
            row,
            rows: match open {
                true => uses.candidates.len(),
                false => 0,
            },
        })
    }

    /// Which `uses` capsule `p` is on, as `(node, input)` — or `None` off every one
    /// of them.
    ///
    /// A press here opens a card and emits nothing, which is [`Load`]'s pulldown
    /// exactly: what a pick asks for is the operation, and *open the list* is not
    /// something a map or a model could ever want to say (ADR-0305).
    pub fn uses_chip(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<(usize, usize)> {
        self.drawn(&pane.nodes).find_map(|node| {
            let at = pane.nodes.get(node)?;
            (0..at.uses.len())
                .find(|index| {
                    self.uses_line(ctx, pane, node, *index, false)
                        .is_some_and(|line| line.hit_chip(p))
                })
                .map(|index| (node, index))
        })
    }

    /// What a press at `p` on an open card asks for:
    /// [`Operation::WireInput`](karakuri_operation::Operation::WireInput) naming
    /// the node the pick landed on — or `None` off every row.
    ///
    /// # It names the node, and the refusal is not here
    ///
    /// The card lists [`Uses::candidates`], which is a reading of the Set somebody
    /// else took, and what leaves this crate is the name that was picked. Nothing
    /// is validated on this side: a name the Set cannot use is refused where the
    /// Set is *built*, by name and with what the Set does hold — the same wall a
    /// model's `wire_input` meets, which sends its edge to the same place with no
    /// check of its own
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// A pick replaces, and that is the language's shape rather than this
    /// control's: an input takes one node, `SetError::SlotBoundTwice` refuses two
    /// edges on one input, and a Set with an unbound input does not build at all
    /// (ADR-0152). So there is no *unwire*, and nothing here offers one.
    pub fn wired(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        viewport: Rect,
        open: (usize, usize),
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let (node, index) = open;
        let at = pane.nodes.get(node)?;
        let uses = at.uses.get(index)?;
        let line = self.uses_line(ctx, pane, node, index, true)?;
        let to = uses.candidates.get(line.picked(viewport, p)?)?;
        Some(Operation::WireInput {
            deck: pane.deck as u8,
            node: at.name.clone(),
            slot: uses.slot.clone(),
            to: to.clone(),
        })
    }

    /// What a press at `p` takes hold of, as the model holds every other fader —
    /// [`grabbed`], so a parameter fader keeps whatever it grabbed at and the value
    /// does not jump under the hand.
    pub fn grab(&self, pane: &Pane, p: karakuri_layout::Point) -> Option<Grab> {
        let grip = self.grip(pane, p)?;
        grabbed(
            grip.fader,
            Knob::Param {
                deck: grip.deck,
                param: grip.param.param.clone(),
                range: grip.param.range,
            },
            Pos2::new(p.x, p.y),
        )
    }
}

/// How tall everything in a pane comes to: [`group_h`] over every node, with a
/// [`size::HAIRLINE`] between two of them.
///
/// One function because two callers must agree. [`pane_box`] clamps the
/// position in force against it and [`View::scroll_by`] clamps the stored one,
/// and the same sum written twice is two answers to how far a pane scrolls.
fn content_h(nodes: &[Node]) -> f32 {
    let mut total = 0.0;
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 {
            total += size::HAIRLINE;
        }
        total += group_h(node);
    }
    total
}

/// One pane of the Inspector, derived — see [`InspectorPane`] for what is drawn
/// here and for the nine things in the mock's pane that are not.
///
/// `index` is which pane, into [`PANE_NAMES`]. `scroll` is the position that
/// pane is scrolled to — [`View::scroll_in`], the console's own state and not a
/// reading — and it is taken here rather than applied by the painter, because a
/// control is hit-tested off the derivation that draws it and an offset added
/// on one side of that seam and not the other is two answers about where a knob
/// is. It arrives unclamped and leaves clamped: [`InspectorPane::scroll`] is
/// what is in force, and nothing is written back (P-0082). `layout` must be
/// solved: [`Layout::rect`](karakuri_layout::Layout::rect) refuses to answer
/// from a dirty one. Like [`library`] this asks `egui` for nothing: every box
/// in the pane is either the full width of the pane or a track of the mock's
/// own grid, so no rectangle here is the width of the type in it.
///
/// `None` where there is no pane by that name, and `None` where there is no
/// room for the two heads — which is [`picture_rect`]'s rule stated on a pane.
/// A console with no deck behind it hands over no panes at all, and what the
/// bay draws then is its card, its head and the bar between the two panes,
/// exactly as it did before this pass — [`mixer`]'s rule, one bay along.
///
/// The bay head is taken off the top here and not in [`pane_box`], which is
/// what [`mixer::strips_row`] and [`library::library_box`] do one bay along:
/// the head is painted *over* the region rather than laid out beside it, so
/// every body in this file starts at `region.min.y + size::HEAD_H` and the
/// arithmetic under it is written as if the head were not there. A pane is the
/// one body in the arrangement whose region is not the bay's own —
/// `inspector-1` is a child of the split — and that is what hid this: the pane
/// is the full height of the bay, head included, so a `.half-head` drawn at
/// `region.min` lands on top of the word `Inspector`.
pub fn inspector(
    layout: &karakuri_layout::Layout,
    index: usize,
    pane: &Pane,
    scroll: f32,
) -> Option<InspectorPane> {
    let region = to_egui(layout.rect(layout.find(PANE_NAMES.get(index)?)?));
    let under_head = Rect::from_min_max(
        Pos2::new(region.min.x, region.min.y + size::HEAD_H),
        region.max,
    );
    pane_box(under_head, &pane.nodes, scroll)
}

/// The arithmetic of a pane, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.half-head { padding: 5px 10px; border-bottom: 1px solid var(--c-hair) }`
///   — a [`size::HALF_HEAD_H`] row along the top of the pane, its rule the
///   bottom pixel of it.
/// - `.deck-head { padding: 5px 10px }` — a [`size::DECK_HEAD_H`] row under
///   it, with no rule of its own: *"Its box is `.node-head`'s without the
///   tint"*, and the tint is what separates it from the group below.
/// - what is left is the node groups', stacked from the top.
///
/// The leftover is the last group's and not the pane's, which is the
/// opposite of what [`library::library_box`] does with its foot, and the reason is the
/// same read the other way: the mock's pane is a flow with nothing under the
/// groups at all, so there is no row for a leftover to sit under. It shows as
/// the bay's own card below the last group, which is every other empty body in
/// this pass.
fn pane_box(region: Rect, nodes: &[Node], scroll: f32) -> Option<InspectorPane> {
    let head = Rect::from_min_max(
        region.min,
        Pos2::new(region.max.x, region.min.y + size::HALF_HEAD_H),
    );
    let deck_head = Rect::from_min_max(
        Pos2::new(region.min.x, head.max.y),
        Pos2::new(region.max.x, head.max.y + size::DECK_HEAD_H),
    );
    let body = Rect::from_min_max(Pos2::new(region.min.x, deck_head.max.y), region.max);
    // **Narrower than a parameter row's own padding is no pane**, which is
    // [`library::library_box`]'s width check with the mock's own indent in it. There is
    // no matching check down the pane: a pane too short for a group draws its
    // two heads and no group, which is what `shown` answers.
    if body.width() <= size::PARAM_PAD_L + size::PARAM_PAD_R {
        return None;
    }
    // **And a pane too short for its two heads is no pane**, which is what
    // this function's caller promises. `positive` is not enough on its own:
    // the two heads are stated heights, so they stay positive while running
    // off the bottom of a region shorter than their sum.
    if !positive(head) || !positive(deck_head) || deck_head.max.y > region.max.y {
        return None;
    }
    // How tall the whole stack is, from the one function `View::scroll_by`
    // clamps the stored position against as well.
    let content = content_h(nodes);
    // **The clamp is here and the store is not touched.** `max(0.0)` is what
    // a pane taller than its content answers — there is nothing to scroll
    // through, so the position in force is the top whatever an operator once
    // spun the wheel to, and the position they spun to is still where they
    // left it when the pane comes back (P-0082, ADR-0250).
    let scroll = scroll.clamp(0.0, (content - body.height()).max(0.0));
    // **How many are whole**, which is the readout's number and not the walk's
    // — `InspectorPane::drawn` is the walk. A group is whole when both its
    // edges are inside the body: the top one after the scroll has been taken
    // off, and the bottom one before the body's own.
    let mut shown = 0;
    let mut top = -scroll;
    for (index, node) in nodes.iter().enumerate() {
        let rule = match index {
            0 => 0.0,
            _ => size::HAIRLINE,
        };
        top += rule;
        let bottom = top + group_h(node);
        if top >= -EPSILON && bottom <= body.height() + EPSILON {
            shown += 1;
        }
        top = bottom;
    }
    Some(InspectorPane {
        head,
        deck_head,
        body,
        scroll,
        content,
        shown,
    })
}

/// What counts as touching an edge, for [`pane_box`]'s *is this group whole* —
/// a hair either way, because both sides of that comparison are sums of `f32`
/// constants and a group that exactly fills the body would otherwise be counted
/// or not by the last bit of a float.
const EPSILON: f32 = 0.001;

/// Whether the deck a pane is showing is on air, off the Mixer bay's own
/// reading of it.
///
/// [`Strip::tally`] through [`residency`], which is the one place a tally
/// becomes a residency on this console — a second reading of it here would be
/// two statements about one fact, and the mock draws the pane's `keep` and the
/// strip's tally in one pink for exactly the reason that they are one fact.
///
/// A deck with no strip is not on air, which is a state rather than a
/// fallback: [`View::mixer`] is as long as the deck has slots, so a pane
/// pointed past the end is pointed at nothing, and nothing is not live.
pub(super) fn on_air(strips: &[Strip], deck: usize) -> bool {
    strips
        .get(deck)
        .is_some_and(|strip| residency(strip.tally) == Residency::Live)
}

/// One pane of the Inspector, painted.
///
/// Where everything goes is [`inspector`]'s, so this paints and derives
/// nothing but the position of one chip after another along a row, which is
/// what a flex row is.
///
/// Term for term from `style.css`:
///
/// - `.half-head` — `color: var(--c-faint)` for the label, `.what`'s
///   `color: var(--c-text)` for the deck and its material, over a
///   `border-bottom: 1px solid var(--c-hair)`.
/// - `.deck-head` — a `.mini` for the sync mode, `.anchor` at
///   [`size::ANCHOR_SIZE`] beside it, and the fold's `.mini` pushed to the
///   right by `.sep`'s `flex: 1`.
/// - `.node-head` — `background: var(--c-tint)`, `.addr`'s
///   `color: var(--c-lav)`, the name in `var(--c-dim)`, and `.auth`'s three
///   words at the right.
/// - `.rend-row` — `.rend` chips, the live one in `var(--c-pink)` over a 15%
///   wash of it.
/// - `.param` — the mock's four tracks, with the fader taking what the other
///   three leave.
///
/// Everything is clipped to the pane, which is what makes the overflow
/// safe: a group that fits and a name that does not are the same clip, and it
/// is the same `with_clip_rect` the picture, a preview cell and the library's
/// list are each drawn inside.
pub(super) fn inspector_into(
    ui: &Ui,
    pal: &Palette,
    at: &InspectorPane,
    pane: &Pane,
    on_air: bool,
    policy: SlotPolicy,
    naming: Option<&str>,
    // **The pulldown's mark**, derived by the caller off the same reading the
    // press is hit-tested against — `View::pane_pulldown`. It is handed in
    // rather than asked here because the card's rows are read off the mixer,
    // which `draw` has already borrowed. The card itself is painted after
    // every bay, for the `uses` line's card's reason.
    target: Option<PaneTarget>,
) {
    // **Derived here and hit-tested by `claim` off the same call**, and asked
    // before the words are painted rather than after: `.half-head` is a flex
    // row with `.sep` between them, so the readout is what gives way when the
    // pane is narrow and the pill keeps its place. `None` is a head with no
    // room for the capsule, which draws none — see [`keep_pill`].
    let keep = keep_pill(ui.ctx(), at, pane);
    let mcp = slot_mcp_pill(ui.ctx(), at, pane, policy);
    // **And the count beside it**, which is what rule 04 asks of a pane that
    // is showing part of itself — derived here off the same head and painted
    // below, exactly as the capsule is. See [`pane_count`].
    let count = pane_count(
        ui.ctx(),
        at,
        pane,
        naming,
        mcp.as_ref().map(|pill| pill.pill),
    );
    // What is left of the head for the two words: everything up to whatever is
    // next along the row, one `.half-head` gap short of it. A name too long for
    // that is clipped, which is the row's own answer to a long name either way
    // — the head is a clip rectangle and there is no ellipsis in this console
    // to draw.
    let words = match count
        .map(|c| c.min.x)
        .or(mcp.map(|pill| pill.pill.min.x))
        .or(keep.map(|pill| pill.pill.min.x))
    {
        Some(x) => Rect::from_min_max(
            at.head.min,
            Pos2::new(x - size::HALF_HEAD_GAP, at.head.max.y),
        ),
        None => at.head,
    };
    let painter = ui.painter().with_clip_rect(words);
    let label = painter.layout_job(span_at(head_label(naming), size::BASE, pal.faint));
    let y = at.head.center().y - label.size().y * 0.5;
    painter.galley(
        Pos2::new(at.head.min.x + size::HALF_HEAD_PAD_X, y),
        label,
        pal.faint,
    );
    // **The run, from the same derivation `claim` hit-tests** — the mock's
    // `.what`, and the console's second letter-taking flow while a name is
    // going into it. `None` is a head with no room to paint any of it, which
    // is [`deck_name`]'s own refusal and leaves the label alone in the row.
    if let Some(named) = deck_name(
        ui.ctx(),
        at,
        pane,
        naming,
        mcp.as_ref().map(|pill| pill.pill),
    ) {
        // **A ground under the field while it is asking, and none while it is
        // reading.** A caret says letters are going *somewhere*; the tint says
        // where, which is the one thing a run of text in a row of readouts
        // cannot say for itself. It is `.node-head`'s own `--c-tint`, so the
        // console spends no new colour on it — and the pink a capsule is lit
        // in is deliberately not reached for here, because that pink means
        // *on air* two controls away.
        let text = match naming {
            Some(typed) => naming_text_in_head(pane, typed),
            None => showing_text(pane),
        };
        editable_text_field(&painter, pal, named.name, &text, naming.is_some(), 3);
    }
    // `.half-head`'s own `border-bottom`, the bottom pixel of the row — drawn
    // through the whole head rather than through the words' clip, which stops
    // one gap short of the pill.
    let painter = ui.painter().with_clip_rect(at.head);
    let rule = at.head.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [
            Pos2::new(at.head.min.x, rule),
            Pos2::new(at.head.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
    // **The pulldown's mark, after the run and inside the head's own clip** —
    // the mock's `▾` beside `deck A · drift_night`, drawn rather than typed
    // for [`CHEVRON_W`]'s reason. It is painted in the label's ink rather than
    // the run's: the mark is a control and the name beside it is a readout,
    // and the console draws every `▾` it has in `--c-faint`.
    if let Some(target) = target {
        let mark = target.chevron;
        chevron_down(&painter, mark, pal.faint);
    }
    // **The mock draws the first pane's `keep` as `.pill.on` and the second
    // pane's as a plain `.pill`**, and what the lit one reads is now on the
    // page: the deck this pane is *showing* is on air. Deck A in the mock is
    // on air *and* holds the selection *and* is the first pane, and the wash
    // is the first of the three for two reasons the console already holds —
    // `.pill.on`'s pink *is* the pink a tally on air is drawn in
    // ([`on_pill_at`]), and the selection is drawn in lavender everywhere
    // else on this panel, so a pink wash meaning *selected* would be the one
    // colour on the console saying two things.
    //
    // **It is handed in rather than asked here**, which is `mixer_into`'s
    // `marked` and `selection` one bay over: residency is the *mixer's*
    // reading of a deck — [`Strip::tally`] — and a second derivation of it in
    // this bay would be two statements about one fact.
    if let Some(pill) = keep {
        match on_air {
            true => on_pill_at(ui, pal, pill.pill, KEEP_LABEL),
            false => pill_at(ui, pal, pill.pill, KEEP_LABEL),
        }
    }
    if let Some(pill) = mcp {
        match policy {
            SlotPolicy::On => armed_pill_at(ui, pal, pill.pill, pill.policy.pill_word()),
            SlotPolicy::Off => pill_at(ui, pal, pill.pill, pill.policy.pill_word()),
            SlotPolicy::Auto => {
                if on_air {
                    pill_at(ui, pal, pill.pill, pill.policy.pill_word());
                } else {
                    armed_pill_at(ui, pal, pill.pill, pill.policy.pill_word());
                }
            }
        }
    }
    // **The count, in the label's own ink**: `.half-head`'s `color:
    // var(--c-faint)`, which is what the mock gives every readout in this row
    // and what the Library foot gives its own `5 of 27`. It is painted inside
    // the head's clip and not the words' — the words stop short of it.
    if let Some(rect) = count {
        let galley = painter.layout_job(span_at(&count_text(at, pane), size::BASE, pal.faint));
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            pal.faint,
        );
    }

    // **Derived here and hit-tested by `claim` off the same call**, which is
    // the rule every other control on this panel is drawn under. `None` is a
    // row too narrow to hold its chips, and it draws none rather than half of
    // each — see [`deck_head`].
    if let Some(head) = deck_head(ui.ctx(), at, pane) {
        deck_head_into(ui, pal, &head, pane);
    }

    // **The clip is what makes a scrolled pane safe**, and it is the same
    // rectangle [`InspectorPane::grip`] refuses a press outside: a group cut
    // by the top edge is painted with its head under the deck head and clipped
    // away there, and a press on the part that is not on screen reaches
    // nothing.
    let painter = ui.painter().with_clip_rect(at.body);
    for index in at.drawn(&pane.nodes) {
        let node = &pane.nodes[index];
        let rect = at.group(&pane.nodes, index);
        node_into(&painter, pal, rect, node);
        // `.node-group`'s `border-bottom: 1px solid var(--c-hair)`, which
        // `:last-child` does not carry — so it goes *between* two groups, and
        // the last node's is not drawn whether or not the pane is scrolled far
        // enough to have it on screen. It is `nodes.len()` and no longer the
        // count of what is drawn, because a group cut by the bottom edge has
        // a rule under it and the next group is what it separates from.
        if index + 1 < pane.nodes.len() {
            let rule = rect.max.y + size::HAIRLINE * 0.5;
            painter.line_segment(
                [Pos2::new(rect.min.x, rule), Pos2::new(rect.max.x, rule)],
                Stroke::new(size::HAIRLINE, pal.hair),
            );
        }
    }
}

impl View {
    /// Which pane head is taking letters, and what is in it — or `None` for a
    /// console where nothing is being named, which is every test in this crate that
    /// does not say otherwise.
    pub fn naming_set(&self) -> Option<&Naming> {
        self.naming.as_ref()
    }

    /// What pane `index`'s head is taking letters into, or `None` where it is
    /// reading. This is what [`deck_name`] and [`inspector_into`] each ask, so that
    /// one head is asking and the other is not.
    pub fn naming_set_in(&self, index: usize) -> Option<&str> {
        self.naming
            .as_ref()
            .filter(|naming| naming.pane == index)
            .map(Naming::typed)
    }

    /// What is being typed into the head of whichever pane is showing `deck`, or
    /// `None` where no head is asking for a name over that deck.
    ///
    /// What it is for is a node's keep (ADR-0338, decision 4): the capsule on a
    /// node group's head types nothing and takes a stamp, and a head that *is*
    /// taking letters is what a keep from that pane files under — which is
    /// ADR-0128's two routes drawn on one capsule, exactly as the deck's own `keep`
    /// draws them.
    ///
    /// The pane is found by the deck rather than carried, which is
    /// [`View::named_set`]'s own rule: the gesture spans frames, so what is filed
    /// is what the head says it is filing *now*. An empty buffer answers
    /// `Some("")`, and that is the field's rule and not this method's — the console
    /// emits what was typed, including nothing, and the wall is where the file is
    /// written (P-0090).
    pub fn naming_over(&self, deck: u8) -> Option<String> {
        let naming = self.naming.as_ref()?;
        let pane = self.inspector.get(naming.pane)?;
        (pane.deck == usize::from(deck)).then(|| naming.typed().to_owned())
    }

    /// Ask for a name in pane `index`'s head, starting from empty.
    ///
    /// Starting from empty rather than from the material's name. The run under the
    /// caret read `drift_night` a moment ago and the field does not keep it: a
    /// buffer seeded with what was there is a name an operator commits by pressing
    /// return once, which is the shape of an overwrite nobody typed. What ADR-0128
    /// makes an instruction is a name that was *typed*.
    pub fn name_set(&mut self, index: usize) {
        self.naming = Some(Naming {
            pane: index,
            typed: String::new(),
        });
    }

    /// Take the field away, typed name and all, which is what escape asks and what
    /// a press somewhere else asks. [`Arrangement::shut`]'s sentence: a name
    /// abandoned half-typed is not kept for the next time, because the buffer is
    /// the gesture and the gesture ended.
    pub fn stop_naming_set(&mut self) {
        self.naming = None;
    }

    /// One character into the name being typed, and `false` where no head was
    /// asking for one. [`Arrangement::typed`]'s rule and its refusal: control
    /// characters are not a name and never reach the buffer, because a newline is
    /// Return arriving as text and that is the commit.
    pub fn type_into_name(&mut self, c: char) -> bool {
        match (&mut self.naming, c.is_control()) {
            (Some(naming), false) => {
                naming.typed.push(c);
                true
            }
            _ => false,
        }
    }

    /// The last character back out again, and `false` where there was nothing to
    /// take — no head asking, or an empty name.
    pub fn rub_out_of_name(&mut self) -> bool {
        match &mut self.naming {
            Some(naming) => naming.typed.pop().is_some(),
            None => false,
        }
    }

    /// The name is finished, and this is what it asks for: the deck that head is
    /// showing, filed under what was typed.
    ///
    /// # The deck is read at the commit and not at the press
    ///
    /// [`KeepPill`] carries the deck it was measured for because its press is one
    /// instant; this gesture spans frames, and what is filed has to be the deck the
    /// head says it is filing *now*. So the pane is looked up again and `None` is a
    /// pane that has gone — a console handed a shorter [`View::inspector`] while
    /// somebody was typing — where the field is taken away and nothing is emitted,
    /// rather than a keep landing on a deck whose head is no longer on screen.
    ///
    /// # It refuses nothing else
    ///
    /// An empty name arrives here as an empty name and leaves as one, which is
    /// [`Arrangement`]'s rule at the same seam: `id` is one path component and the
    /// wall is where the bytes are written. A name typed twice overwrites, which is
    /// ADR-0128 and is not this control's to soften.
    ///
    /// The field is taken away whether or not the name is any good, for
    /// `Readout::named`'s reason one bay along: a refusal is said out loud by
    /// whoever refuses it, and a field left standing over the refusal would be the
    /// panel asking the question again without saying the answer.
    pub fn named_set(&mut self) -> Option<Operation> {
        let naming = self.naming.take()?;
        let deck = self.inspector.get(naming.pane)?.deck as u8;
        Some(Operation::SaveSet {
            deck,
            id: Some(naming.typed),
        })
    }

    /// Which `uses` line's card is down, or `None` — see [`View::wiring_open`] the
    /// field.
    pub fn wiring_open(&self) -> Option<(usize, usize, usize)> {
        self.wiring_open
    }

    /// Put one `uses` line's card down, and answer whether anything moved.
    ///
    /// Refused for a console with no pane, which is [`View::open_target`]'s own
    /// guard and its reason: a card with no line under it offers nothing to pick
    /// and nothing to leave by, and `input::claim`'s rule 2 would hand it every
    /// press until a second one shut it.
    pub fn open_wiring(&mut self, pane: usize, node: usize, input: usize) -> bool {
        if self.inspector.get(pane).is_none() {
            return false;
        }
        let at = Some((pane, node, input));
        let moved = self.wiring_open != at;
        self.wiring_open = at;
        moved
    }

    /// Put it away, and answer whether one was down.
    pub fn shut_wiring(&mut self) -> bool {
        let was = self.wiring_open.is_some();
        self.wiring_open = None;
        was
    }

    /// How far one Inspector pane is scrolled, as it is stored — the number
    /// [`inspector`] clamps and never the one it clamped.
    ///
    /// Zero for a pane index past [`PANES`], which is a caller's error and not a
    /// state: the bay has two panes and `PANE_NAMES` is what says so.
    pub fn scroll_in(&self, pane: usize) -> f32 {
        self.scroll.get(pane).copied().unwrap_or(0.0)
    }

    /// Turn one pane's wheel by `by` pixels, positive down the list, and answer
    /// whether the stored position moved.
    ///
    /// # Two clamps, and only one of them is here
    ///
    /// This one is against the content — how tall everything the deck publishes
    /// comes to — and it is a reading of the deck rather than of a viewport, so a
    /// stored position bounded by it is not a position any resize can rewrite.
    /// Without it a wheel spun over a short Set would put the number in the
    /// thousands and an operator would have to spin it all the way back before
    /// anything moved, which is *"a control you cannot see being moved"* by another
    /// name.
    ///
    /// The other clamp is against the pane's own height and belongs where the pane
    /// is laid out — [`InspectorPane::scroll`], which is
    /// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md): a
    /// shorter pane draws less of the same position and stores nothing, so dragging
    /// it back reproduces the picture exactly rather than nearly (ADR-0250's
    /// argument one region in).
    ///
    /// A pane this console is not showing anything in refuses the wheel rather than
    /// storing a position for it, which is [`View::point_at`]'s rule: what a
    /// pointer can be at is something drawn.
    pub fn scroll_by(&mut self, pane: usize, by: f32) -> bool {
        let Some(showing) = self.inspector.get(pane) else {
            return false;
        };
        let Some(at) = self.scroll.get_mut(pane) else {
            return false;
        };
        let content = content_h(&showing.nodes);
        let next = (*at + by).clamp(0.0, content);
        let moved = next != *at;
        *at = next;
        moved
    }

    /// Which deck one Inspector pane is pointed at — see [`View::pane_deck`] the
    /// field, which is where the argument is.
    ///
    /// This is what the host reads to fill the pane, which is
    /// [`View::target_deck`]'s arrangement one bay along: the pointer is the
    /// console's and what is under it is the host's answer to *what is that deck
    /// playing*.
    ///
    /// Deck A for a pane index past [`PANES`], which is a caller's error and not a
    /// state — [`View::scroll_in`]'s own rule.
    ///
    /// [`View::pane_deck`]: Self::pane_deck
    pub fn pane_deck(&self, pane: usize) -> u8 {
        self.pane_deck.get(pane).copied().unwrap_or(0)
    }

    /// Every pane's target at once, which is what a host reads before it fills
    /// [`View::inspector`]: the panes are written through a `&mut` of that field,
    /// so asking pane by pane while it is borrowed is a second borrow of this
    /// struct. It is two bytes and `Copy`.
    pub fn pane_decks(&self) -> [u8; PANES] {
        self.pane_deck
    }

    /// Point one pane at `deck`, put the card away, and answer whether anything
    /// moved.
    ///
    /// A deck the mixer has no strip for is refused, which is [`View::select`]'s
    /// rule and [`View::aim_at`]'s read a third time rather than a third rule: the
    /// head says which deck it is showing, so a target past the deck's slots would
    /// be a letter naming a deck with nothing under it.
    ///
    /// It refuses rather than clamping, for `select`'s reason: a pick of deck D at
    /// a two-slot deck means *deck D*, and clamping would point the pane at deck B
    /// — a different deck than the one asked for.
    ///
    /// Nothing else moves, and that is the whole of what this mark is for: not the
    /// deck selection, not the pane next door, not the Library bay's load target.
    /// Each of those is a pointer of its own with a writer of its own, and this one
    /// touches none of them.
    ///
    /// The card goes away here, which is [`View::aim_at`]'s clause: a pick is one
    /// gesture and this is the whole of it, so a card left down would go on
    /// claiming every press on the console. It is put away even where the pane did
    /// not move — picking the deck a pane already shows is still a hand finishing
    /// what it started.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a move and not on a
    /// press.
    pub fn point_pane(&mut self, pane: usize, deck: u8) -> bool {
        if usize::from(deck) >= self.mixer.len() {
            return false;
        }
        let Some(at) = self.pane_deck.get_mut(pane) else {
            return false;
        };
        let moved = *at != deck || self.pane_open.is_some();
        *at = deck;
        self.pane_open = None;
        moved
    }

    /// Which pane head's pulldown is down, or `None` — see [`View::pane_open`] the
    /// field.
    ///
    /// [`View::pane_open`]: Self::pane_open
    pub fn pane_target_open(&self) -> Option<usize> {
        self.pane_open
    }

    /// Put one pane head's card down, and answer whether anything moved.
    ///
    /// Refused for a pane this console is not showing, which is
    /// [`View::open_wiring`]'s own guard and its reason: a card with no head under
    /// it offers nothing to pick and nothing to leave by, and `input::claim`'s rule
    /// 2 would hand it every press until a second one shut it.
    pub fn open_pane_target(&mut self, pane: usize) -> bool {
        if pane >= PANES || self.inspector.get(pane).is_none() {
            return false;
        }
        let moved = self.pane_open != Some(pane);
        self.pane_open = Some(pane);
        moved
    }

    /// Put it away, and answer whether one was down.
    pub fn shut_pane_target(&mut self) -> bool {
        let was = self.pane_open.is_some();
        self.pane_open = None;
        was
    }

    /// What one pane head's pulldown is, laid out — the mark and, while it is down,
    /// the card under it.
    ///
    /// Read once for the frame and handed to the paint and to the press, exactly as
    /// [`View::target`] is: the mark that is drawn and the mark a press lands on
    /// are one derivation of one reading.
    ///
    /// `None` is a head with no run in it, which is [`deck_name`]'s refusal — see
    /// [`pane_target`].
    pub fn pane_pulldown(
        &self,
        ctx: &egui::Context,
        at: &InspectorPane,
        pane: &Pane,
        index: usize,
    ) -> Option<PaneTarget> {
        let policy = self
            .slot_policies
            .get(pane.deck)
            .copied()
            .unwrap_or_default();
        let mcp = slot_mcp_pill(ctx, at, pane, policy).map(|p| p.pill);
        pane_target(
            ctx,
            at,
            pane,
            index,
            self.naming_set_in(index),
            self.mixer.len(),
            self.pane_open == Some(index),
            mcp,
        )
    }
}

#[cfg(test)]
mod tests {
    //! What is private and worth asserting, and it is in `src/` rather than in
    //! `tests/` for that one reason.
    //!
    //! Every region of this console is asserted from a file in `tests/`, one per
    //! region, and everything here would have been in one of them. It is here
    //! because what it names is private, and moving it out would mean widening a
    //! surface to test it — a wider surface for a narrower reason.
    //!
    //! Moved here from `view/mod.rs`, on [`transport`]'s own precedent:
    //! [`pane_box`] and [`group_h`] are the pane's arithmetic, and the public
    //! [`inspector`] is that arithmetic plus a layout lookup — the reasoning that
    //! was already written here travels with the code under
    //! [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md),
    //! the same rule `offset_text`'s own assertion moved under, one bay over.
    //!
    //! What is *not* here and belongs in `tests/inspector.rs` when somebody writes
    //! it: the bay drawn against a real arrangement at the two viewports, on
    //! `tests/library.rs`'s pattern.

    use super::*;

    /// A pane with `params` parameters under one node and no renderers.
    fn one_node(params: usize) -> Pane {
        Pane {
            deck: 0,
            material: "drift_shell + soft_points".to_owned(),
            sync: Sync::Tempo,
            // Nothing refused, which is what a pane about the arithmetic of
            // its rows says about material it is not asking after.
            allows: [true; SYNCS.len()],
            anchor_bpm: 128.0,
            scrub_beats: 0.0,
            composite: false,
            // Not this module's row: the deck head's two build chips are drawn
            // from this, and everything here is about the rows under it.
            aimed: None,
            nodes: vec![Node {
                addr: "L1:0".to_owned(),
                name: "drift_shell".to_owned(),
                authority: Some(NodeAuthority {
                    at: NodeAddress {
                        layer: Layer::L1,
                        index: 0,
                    },
                    level: Authority::Manual,
                }),
                // **A node with a source**, which every node but the built-in
                // camera has — and the capsule this draws is what the tests
                // below measure the head's right-hand end against.
                keep: Some(NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                }),
                uses: Vec::new(),
                renderers: Vec::new(),
                params: (0..params)
                    .map(|n| Param {
                        ord: Some(n + 1),
                        name: format!("p{n}"),
                        value: 0.5,
                        range: [0.0, 1.0],
                        param: karakuri_operation::ParamAt {
                            node: None,
                            key: format!("p{n}"),
                        },
                        bound: None,
                    })
                    .collect(),
            }],
        }
    }

    /// A rectangle the size of one inspector pane at the narrowest console the mock
    /// will draw: `.console`'s `min-width: 1010px` holds the centre at 484 and each
    /// pane at 237.
    fn pane_at(height: f32) -> Rect {
        Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(237.0, height))
    }

    /// The two heads are the mock's boxes and the leftover is the groups'.
    ///
    /// `.half-head` is `5 + 16.5 + 5` over its own `border-bottom`, and
    /// `.deck-head` is `5 + 15.5 + 5` with none — the stylesheet's own arithmetic
    /// for that row. Everything under them is the body.
    #[test]
    fn a_pane_is_two_heads_and_what_is_left() {
        let pane = one_node(2);
        let at = pane_box(pane_at(400.0), &pane.nodes, 0.0).expect("a pane with room in it");
        assert_eq!(at.head.height(), 27.5);
        assert_eq!(at.deck_head.height(), 25.5);
        assert_eq!(at.head.max.y, at.deck_head.min.y);
        assert_eq!(at.deck_head.max.y, at.body.min.y);
        assert_eq!(at.body.max.y, 400.0);
    }

    /// The inspector's own minimum is the least this draws, and the two are one
    /// derivation: a pane at the minimum less the bay head has room for exactly the
    /// one group of two parameters the minimum is written from, and a pixel less
    /// has room for none of it.
    ///
    /// This is what the 151.5 in `lib.rs` *means*: bay head 27, `.half-head` 27.5,
    /// `.deck-head` 25.5, `.node-head` 26.5 and two `.param` rows at 22.5.
    #[test]
    fn the_bays_minimum_is_the_least_a_pane_can_draw() {
        let pane = one_node(2);
        let pane_h = 151.5 - size::HEAD_H;
        let at = pane_box(pane_at(pane_h), &pane.nodes, 0.0).expect("a pane at the minimum");
        assert_eq!(
            at.shown, 1,
            "one node and two of its parameters is what the minimum is written from"
        );
        let short = pane_box(pane_at(pane_h - 0.5), &pane.nodes, 0.0).expect("still two heads");
        assert_eq!(
            short.shown, 0,
            "half a pixel under the minimum and the group no longer fits"
        );
    }

    /// A group the pane cannot hold whole is not counted as shown, which is what
    /// the head's `n of m` means since the pane started scrolling: it is drawn, as
    /// far as the pane goes, and it is not one of the ones the readout says are on
    /// screen.
    #[test]
    fn a_group_that_does_not_fit_whole_is_not_counted() {
        let pane = Pane {
            nodes: vec![one_node(2).nodes[0].clone(), one_node(5).nodes[0].clone()],
            ..one_node(2)
        };
        // Room for the first group, the hairline, and all but the last row of
        // the second.
        let first = size::NODE_HEAD_H + size::PARAM_H * 2.0;
        let second = size::NODE_HEAD_H + size::PARAM_H * 5.0;
        let body = first + size::HAIRLINE + second - size::PARAM_H;
        let at = pane_box(
            pane_at(size::HALF_HEAD_H + size::DECK_HEAD_H + body),
            &pane.nodes,
            0.0,
        )
        .expect("a pane with room in it");
        assert_eq!(
            at.shown, 1,
            "the second group is short by one row, so one is whole"
        );

        // One row more and both are drawn.
        let at = pane_box(
            pane_at(size::HALF_HEAD_H + size::DECK_HEAD_H + body + size::PARAM_H),
            &pane.nodes,
            0.0,
        )
        .expect("a pane with room in it");
        assert_eq!(at.shown, 2);
    }

    /// Where a group goes is the sum of the groups above it and the rules between
    /// them, and the rule is between rather than under: *n* groups carry *n - 1* of
    /// them, because `.node-group:last-child` has none.
    #[test]
    fn a_group_stacks_under_the_one_before_it_with_a_rule_between() {
        let pane = Pane {
            nodes: vec![
                one_node(2).nodes[0].clone(),
                one_node(1).nodes[0].clone(),
                one_node(3).nodes[0].clone(),
            ],
            ..one_node(2)
        };
        let at = pane_box(pane_at(400.0), &pane.nodes, 0.0).expect("a pane with room in it");
        assert_eq!(at.shown, 3);
        let first = at.group(&pane.nodes, 0);
        let second = at.group(&pane.nodes, 1);
        let third = at.group(&pane.nodes, 2);
        assert_eq!(first.min.y, at.body.min.y);
        assert_eq!(second.min.y, first.max.y + size::HAIRLINE);
        assert_eq!(third.min.y, second.max.y + size::HAIRLINE);
        assert_eq!(first.height(), size::NODE_HEAD_H + size::PARAM_H * 2.0);
        assert_eq!(second.height(), size::NODE_HEAD_H + size::PARAM_H);
    }

    /// A renderer row costs a group its own height, and a group without one costs
    /// nothing: the mock draws `.rend-row` in the `L4` group and nowhere else.
    #[test]
    fn a_renderer_row_is_a_row_of_the_group_it_is_in() {
        let mut node = one_node(1).nodes[0].clone();
        let without = group_h(&node);
        node.renderers = vec![Renderer {
            name: "soft_points".to_owned(),
            live: true,
        }];
        assert_eq!(group_h(&node), without + size::REND_ROW_H);
    }

    /// The anchor is the mock's own two spellings, and free shows neither.
    ///
    /// *"`T128` is the tempo a deck was engaged at … `B128 +0.25` is that with the
    /// deck sitting a quarter beat ahead of the room. A free deck shows neither."*
    #[test]
    fn the_anchor_reads_what_the_mock_reads() {
        let mut pane = one_node(1);
        pane.sync = Sync::Tempo;
        pane.scrub_beats = 0.25;
        assert_eq!(
            anchor_text(&pane).as_deref(),
            Some("T128"),
            "tempo sync does not read the offset, so it is not drawn"
        );
        pane.sync = Sync::Beat;
        assert_eq!(anchor_text(&pane).as_deref(), Some("B128 +0.25"));
        pane.sync = Sync::Free;
        assert_eq!(
            anchor_text(&pane),
            None,
            "free is the absence of a transport rather than a setting"
        );
    }

    /// The pane head names the deck it is pointed at and what is in it, which is
    /// the mock's `deck A · drift_night`.
    #[test]
    fn the_pane_head_says_which_deck_it_is_showing() {
        let mut pane = one_node(1);
        pane.deck = 1;
        assert_eq!(showing_text(&pane), "deck B · drift_shell + soft_points");
    }

    /// Every level the vocabulary names is on the node head.
    ///
    /// `karakuri_operation::Authority` carries no `ALL` — *"it arrives with the
    /// first reader"* — so [`AUTHORITIES`] is this crate's list, and the thing that
    /// can go wrong is the list falling behind the vocabulary while [`auth_word`]
    /// is updated. This holds the two together: every level [`auth_word`] can spell
    /// is in the array exactly once, and the array is in the vocabulary's own
    /// declaration order.
    #[test]
    fn every_authority_the_vocabulary_names_is_on_the_node_head() {
        // A `match` that a fourth level would not compile past, which is what
        // makes this a check on the *array* rather than on the enum.
        let expected = [
            Authority::Manual,
            Authority::Suggesting,
            Authority::Automatic,
        ];
        assert_eq!(
            AUTHORITIES.len(),
            expected.len(),
            "a level the vocabulary names is missing from the node head"
        );
        for (n, level) in expected.into_iter().enumerate() {
            assert_eq!(AUTHORITIES[n], level, "the three are in declaration order");
            assert!(
                !auth_word(level).is_empty(),
                "every level has the console's own abbreviation for it"
            );
        }
        let words: Vec<&str> = AUTHORITIES.into_iter().map(auth_word).collect();
        assert_eq!(
            words,
            vec!["man", "sug", "auto"],
            "the manual's node head reads `man / sug / auto`"
        );
    }

    /// A pane narrower than a parameter row's own padding is no pane, which is the
    /// picture's rule stated across the axis.
    #[test]
    fn a_pane_with_no_room_across_it_draws_nothing() {
        let pane = one_node(2);
        let narrow = Rect::from_min_size(
            Pos2::new(0.0, 0.0),
            egui::vec2(size::PARAM_PAD_L + size::PARAM_PAD_R, 400.0),
        );
        assert!(pane_box(narrow, &pane.nodes, 0.0).is_none());
    }

    /// There are two panes and there is no third.
    ///
    /// The mock's `2 up ▾` is an operator choosing how many while running, and a
    /// [`Spec`](karakuri_layout::Spec) builds a [`Layout`](karakuri_layout::Layout)
    /// once — so the count is the arrangement's, and a caller asking for a pane it
    /// has not got gets `None` rather than one of the two it has.
    ///
    /// And a console with no deck behind it hands over no pane at all, which is
    /// every test in this crate.
    #[test]
    fn there_are_two_panes_and_no_third() {
        let mut layout = crate::layout();
        layout.set_viewport(karakuri_layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        });
        layout.solve();
        let pane = one_node(2);
        assert_eq!(PANES, 2, "the mock's `.insp-split` is `1fr 9px 1fr`");
        for index in 0..PANES {
            assert!(
                inspector(&layout, index, &pane, 0.0).is_some(),
                "pane {index} is in the arrangement and has room in it"
            );
        }
        assert!(
            inspector(&layout, PANES, &pane, 0.0).is_none(),
            "a third pane is a pane the arrangement has not got"
        );
        assert!(View::new(Room::Day).inspector.is_empty());
    }

    /// A pane starts under the bay head and not on top of it.
    ///
    /// The one thing `pane_box`'s own tests cannot see. They are handed a rectangle
    /// and the head has already been taken off it — the fixture says so in the
    /// arithmetic, `151.5 - size::HEAD_H` — so every one of them passes whether or
    /// not the caller does the subtraction. It did not: `.half-head` was drawn at
    /// the top of `inspector-1`, which is the top of the bay, which is where
    /// [`bay_head`] paints the word `Inspector`. Read against the arrangement
    /// rather than against a fixture, because the fixture is what could not tell.
    #[test]
    fn a_pane_starts_under_the_bay_head() {
        let mut layout = crate::layout();
        layout.set_viewport(karakuri_layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        });
        layout.solve();
        let pane = one_node(2);
        let bay = to_egui(layout.rect(layout.find("inspector").expect("the bay is named")));
        for index in 0..PANES {
            let at = inspector(&layout, index, &pane, 0.0).expect("a pane with room in it");
            assert_eq!(
                at.head.min.y,
                bay.min.y + size::HEAD_H,
                "pane {index} starts where the bay head ends"
            );
            assert!(
                bay.contains_rect(at.head) && bay.contains_rect(at.deck_head),
                "pane {index} draws inside the bay it is in"
            );
        }
    }
}
