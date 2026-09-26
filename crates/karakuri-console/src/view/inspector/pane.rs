use super::*;

/// Solved Inspector pane layout: pane head, deck head, and stacked node groups.
///
/// Implements value-driven layout per [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md), [ADR-0314], and [ADR-0218].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectorPane {
    /// `.half-head`, along the top of the pane, with its rule on the bottom.
    pub head: Rect,
    /// `.deck-head`, under it — the deck's own clock and its fold.
    pub deck_head: Rect,
    /// What is left under the two heads, where the node groups stack from the top
    /// with a hairline between them.
    pub body: Rect,
    /// Scroll position in force for this pane, clamped to available content.
    pub scroll: f32,
    /// Total content height of all node groups in this pane.
    pub content: f32,
    /// Number of node groups fully visible in this pane (the `n` in `n of m`).
    ///
    /// See ADR-0307 for pane scrolling and visible count rules.
    pub shown: usize,
}

impl InspectorPane {
    /// Returns the bounding rectangle for group `index` within the pane.
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

    /// Range of group indices visible within the pane's viewport.
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

    /// Hit-tests renderer selection chips at `p`, returning [`Operation::SelectRenderer`] per [P-0090].
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

    /// Hit-tests authority chips at `p`, returning [`Operation::SetAuthority`] or `None`.
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

    /// Hit-tests node keep capsule at `p`, returning [`Operation::SaveSet`] with timestamp per [ADR-0128].
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

    /// Hit-tests sensitivity row controls at `p`, or returns `None` for readouts/unbound rows.
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

    /// Hit-tests a press at point `p` against draggable parameter fader handles.
    ///
    /// Returns `Some(ParamGrip)` if a handle was hit within the visible body, or `None`.
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

    /// Hit-tests a press at point `p` on a parameter publish toggle mark.
    ///
    /// Returns `Some(Operation::Publish)` with the updated ordered interface list, or `None`.
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

    /// Bounding rectangle for the publish indicator of parameter row `row` in group `node`.
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

    /// Control bounding rectangle for input `index` on node `group`, or `None`.
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

    /// Returns `(node, input)` if `p` lands on a `uses` capsule per ADR-0305.
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

    /// Hit-tests candidate list on an open card at `p`, returning [`Operation::WireInput`] per [P-0090] and ADR-0152.
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
