use super::*;

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
